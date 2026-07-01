//! Self-hosted authentication — replaces AWS Cognito.
//!
//! Users + password hashes live in SQLite (the AWS version kept passwords only
//! in Cognito). JWTs are minted AND verified here with a real HS256 signature —
//! the AWS Lambdas shipped with `insecure_disable_signature_validation()`, so any
//! forged token was accepted. That hole is closed.

use std::time::Duration;

use argon2::password_hash::rand_core::{OsRng, RngCore};
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use axum::async_trait;
use axum::extract::{FromRequestParts, State};
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::Json;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::db::{now_unix, User};
use crate::error::{AppError, AppResult};
use crate::AppState;

// ── Password hashing (Argon2id) ─────────────────────────────────────────────

pub fn hash_password(pw: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(pw.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| AppError::Internal(format!("password hash: {e}")))
}

pub fn verify_password(pw: &str, hash: &str) -> bool {
    match PasswordHash::new(hash) {
        Ok(parsed) => Argon2::default()
            .verify_password(pw.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}

// ── JWT ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // user_id
    pub username: String,
    pub email: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    pub token_use: String, // "access" | "refresh"
    pub iat: i64,
    pub exp: i64,
}

pub fn mint(secret: &str, user: &User, token_use: &str, ttl: Duration) -> AppResult<String> {
    let now = now_unix();
    let claims = Claims {
        sub: user.user_id.clone(),
        username: user.username.clone(),
        email: user.email.clone(),
        role: user.role.clone(),
        token_use: token_use.to_string(),
        iat: now,
        exp: now + ttl.as_secs() as i64,
    };
    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(format!("jwt encode: {e}")))
}

/// Verify signature + expiry, and require the expected `token_use`.
pub fn verify(secret: &str, token: &str, expect_use: &str) -> AppResult<Claims> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true; // real verification — signature is always checked
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map_err(|_| AppError::Unauthorized)?;
    if data.claims.token_use != expect_use {
        return Err(AppError::Unauthorized);
    }
    Ok(data.claims)
}

// ── Extractors ──────────────────────────────────────────────────────────────

/// Any authenticated user (valid access token). Role is intentionally NOT carried
/// here — admin status is checked against the DB (see `AdminUser`) so a stale
/// token can't grant privileges.
pub struct AuthUser {
    pub user_id: String,
}

fn bearer(parts: &Parts) -> AppResult<String> {
    let h = parts
        .headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::Unauthorized)?;
    h.strip_prefix("Bearer ")
        .or_else(|| h.strip_prefix("bearer "))
        .map(str::to_string)
        .ok_or(AppError::Unauthorized)
}

#[async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, AppError> {
        let token = bearer(parts)?;
        let claims = verify(&state.cfg.jwt_secret, &token, "access")?;
        Ok(AuthUser {
            user_id: claims.sub,
        })
    }
}

/// An admin (valid access token AND `role == "admin"` in the users table).
pub struct AdminUser(#[allow(dead_code)] pub AuthUser);

#[async_trait]
impl FromRequestParts<AppState> for AdminUser {
    type Rejection = AppError;
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, AppError> {
        let auth = AuthUser::from_request_parts(parts, state).await?;
        // Role is authoritative in the DB, not the token, so a stale token can't
        // grant admin (mirrors the AWS version's DynamoDB role check).
        let user = state.db.get_user_by_id(&auth.user_id).await?;
        if !user.is_admin() {
            return Err(AppError::Forbidden);
        }
        Ok(AdminUser(auth))
    }
}

// ── DTOs (must match frontend/app.js expectations) ──────────────────────────

#[derive(Deserialize)]
pub struct SignupRequest {
    pub email: String,
    pub password: String,
    pub username: String,
}
#[derive(Serialize)]
pub struct SignupResponse {
    pub user_id: String,
    pub email: String,
    pub username: String,
    pub message: String,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String, // username or email
    pub password: String,
}
#[derive(Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub id_token: String,
    pub expires_in: i64,
    pub token_type: String,
}

#[derive(Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}
#[derive(Serialize)]
pub struct RefreshResponse {
    pub access_token: String,
    pub id_token: String,
    pub expires_in: i64,
    pub token_type: String,
}

#[derive(Deserialize)]
pub struct ForgotPasswordRequest {
    pub username: String,
}
#[derive(Deserialize)]
pub struct ConfirmForgotPasswordRequest {
    pub username: String,
    pub confirmation_code: String,
    pub new_password: String,
}
#[derive(Serialize)]
pub struct MessageResponse {
    pub message: String,
}

#[derive(Serialize)]
pub struct UserProfile {
    pub user_id: String,
    pub email: String,
    pub username: String,
    pub display_name: String,
    pub created_at: String, // RFC3339
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}
#[derive(Serialize)]
pub struct MeResponse {
    pub profile: UserProfile,
}

fn rfc3339(unix: i64) -> String {
    chrono::DateTime::from_timestamp(unix, 0)
        .map(|d| d.to_rfc3339())
        .unwrap_or_else(|| "unknown".to_string())
}

// ── Handlers ────────────────────────────────────────────────────────────────

pub async fn signup(
    State(state): State<AppState>,
    Json(req): Json<SignupRequest>,
) -> AppResult<(StatusCode, Json<SignupResponse>)> {
    if req.email.trim().is_empty() || req.password.is_empty() || req.username.trim().is_empty() {
        return Err(AppError::BadRequest(
            "email, username and password are required".into(),
        ));
    }
    if req.password.len() < 8 {
        return Err(AppError::BadRequest(
            "password must be at least 8 characters".into(),
        ));
    }
    if state.db.username_exists(&req.username).await? {
        return Err(AppError::Conflict("username already taken".into()));
    }
    if state.db.email_exists(&req.email).await? {
        return Err(AppError::Conflict(
            "this email is already registered".into(),
        ));
    }

    let user = User {
        user_id: uuid::Uuid::new_v4().to_string(),
        email: req.email.trim().to_string(),
        username: req.username.trim().to_string(),
        display_name: req.username.trim().to_string(),
        password_hash: hash_password(&req.password)?,
        role: None,
        created_at: now_unix(),
    };
    state.db.create_user(&user).await?;

    Ok((
        StatusCode::CREATED,
        Json(SignupResponse {
            user_id: user.user_id,
            email: user.email,
            username: user.username,
            message: "user created successfully".into(),
        }),
    ))
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> AppResult<Json<LoginResponse>> {
    // Uniform error whether the user is missing or the password is wrong.
    let user = state
        .db
        .get_user_by_login(&req.username)
        .await?
        .filter(|u| verify_password(&req.password, &u.password_hash))
        .ok_or_else(|| AppError::BadRequest("invalid username or password".into()))?;

    let secret = &state.cfg.jwt_secret;
    let access = mint(secret, &user, "access", state.cfg.access_token_ttl)?;
    let refresh = mint(secret, &user, "refresh", state.cfg.refresh_token_ttl)?;
    Ok(Json(LoginResponse {
        access_token: access.clone(),
        refresh_token: refresh,
        id_token: access, // frontend only uses access_token; keep the field for compat
        expires_in: state.cfg.access_token_ttl.as_secs() as i64,
        token_type: "Bearer".into(),
    }))
}

pub async fn refresh(
    State(state): State<AppState>,
    Json(req): Json<RefreshRequest>,
) -> AppResult<Json<RefreshResponse>> {
    let claims = verify(&state.cfg.jwt_secret, &req.refresh_token, "refresh")?;
    let user = state.db.get_user_by_id(&claims.sub).await?;
    let access = mint(
        &state.cfg.jwt_secret,
        &user,
        "access",
        state.cfg.access_token_ttl,
    )?;
    Ok(Json(RefreshResponse {
        access_token: access.clone(),
        id_token: access,
        expires_in: state.cfg.access_token_ttl.as_secs() as i64,
        token_type: "Bearer".into(),
    }))
}

pub async fn forgot_password(
    State(state): State<AppState>,
    Json(req): Json<ForgotPasswordRequest>,
) -> AppResult<Json<MessageResponse>> {
    // Always return the same message so we don't leak which usernames exist.
    if let Some(user) = state.db.get_user_by_login(&req.username).await? {
        let code = format!("{:08x}", OsRng.next_u32());
        let code_hash = hash_password(&code)?;
        let expires = now_unix() + 15 * 60; // 15 minutes
        state
            .db
            .upsert_password_reset(&user.user_id, &code_hash, expires)
            .await?;
        // No SMTP on a home instance yet — surface the code in the server log for
        // the admin/user to relay. TODO: send via email.
        tracing::warn!(
            "password reset code for '{}' ({}): {}",
            user.username,
            user.email,
            code
        );
    }
    Ok(Json(MessageResponse {
        message: "if this user exists, a password reset code has been issued".into(),
    }))
}

pub async fn confirm_forgot_password(
    State(state): State<AppState>,
    Json(req): Json<ConfirmForgotPasswordRequest>,
) -> AppResult<Json<MessageResponse>> {
    if req.new_password.len() < 8 {
        return Err(AppError::BadRequest(
            "new password must be at least 8 characters".into(),
        ));
    }
    let user = state
        .db
        .get_user_by_login(&req.username)
        .await?
        .ok_or_else(|| AppError::BadRequest("invalid reset request".into()))?;
    let (code_hash, expires_at) = state
        .db
        .get_password_reset(&user.user_id)
        .await?
        .ok_or_else(|| AppError::BadRequest("invalid reset request".into()))?;

    if now_unix() > expires_at || !verify_password(&req.confirmation_code, &code_hash) {
        return Err(AppError::BadRequest("invalid or expired reset code".into()));
    }

    state
        .db
        .update_password(&user.user_id, &hash_password(&req.new_password)?)
        .await?;
    state.db.delete_password_reset(&user.user_id).await?;
    Ok(Json(MessageResponse {
        message: "password has been reset".into(),
    }))
}

pub async fn me(State(state): State<AppState>, auth: AuthUser) -> AppResult<Json<MeResponse>> {
    let user = state.db.get_user_by_id(&auth.user_id).await?;
    Ok(Json(MeResponse {
        profile: UserProfile {
            user_id: user.user_id,
            email: user.email,
            username: user.username,
            display_name: user.display_name,
            created_at: rfc3339(user.created_at),
            role: user.role,
        },
    }))
}
