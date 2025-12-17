use lambda_http::{Body, Request, Response};
use tracing::info;

use crate::cognito::CognitoService;
use crate::dynamodb_service::DynamoDbService;
use crate::error::{AuthError, AuthResult};
use crate::jwt_auth::JwtValidator;
use crate::models::*;

/// Handle signup request
pub async fn handle_signup(
    request: Request,
    cognito: &CognitoService,
    dynamodb: &DynamoDbService,
) -> AuthResult<Response<Body>> {
    info!("Handling signup request");

    // Parse request body
    let body = request.body();
    let signup_req: SignupRequest = serde_json::from_slice(body.as_ref())
        .map_err(|e| AuthError::InvalidRequest(format!("Invalid JSON: {}", e)))?;

    // Validate input
    if signup_req.email.is_empty() {
        return Err(AuthError::InvalidRequest("Email is required".to_string()));
    }
    if signup_req.password.is_empty() {
        return Err(AuthError::InvalidRequest("Password is required".to_string()));
    }
    if signup_req.username.is_empty() {
        return Err(AuthError::InvalidRequest("Username is required".to_string()));
    }

    // Sign up user in Cognito
    let user_id = cognito
        .sign_up(&signup_req.email, &signup_req.password, &signup_req.username)
        .await?;

    // Auto-confirm user for testing (remove in production)
    cognito
        .admin_confirm_sign_up(&signup_req.username)
        .await?;

    // Create user record in DynamoDB
    let user_record = UserRecord::new(user_id.clone(), signup_req.email.clone(), signup_req.username.clone());
    dynamodb.create_user(&user_record).await?;

    // Build response
    let response = SignupResponse {
        user_id,
        email: signup_req.email,
        username: signup_req.username,
        message: "User created successfully. Email confirmed for testing.".to_string(),
    };

    let response_body = serde_json::to_string(&response)
        .map_err(|e| AuthError::InternalError(format!("Failed to serialize response: {}", e)))?;

    Ok(Response::builder()
        .status(201)
        .header("Content-Type", "application/json")
        .body(response_body.into())
        .unwrap())
}

/// Handle login request
pub async fn handle_login(
    request: Request,
    cognito: &CognitoService,
) -> AuthResult<Response<Body>> {
    info!("Handling login request");

    // Parse request body
    let body = request.body();
    let login_req: LoginRequest = serde_json::from_slice(body.as_ref())
        .map_err(|e| AuthError::InvalidRequest(format!("Invalid JSON: {}", e)))?;

    // Validate input
    if login_req.username.is_empty() {
        return Err(AuthError::InvalidRequest("Username is required".to_string()));
    }
    if login_req.password.is_empty() {
        return Err(AuthError::InvalidRequest("Password is required".to_string()));
    }

    // Authenticate with Cognito
    let tokens = cognito
        .authenticate(&login_req.username, &login_req.password)
        .await?;

    // Build response
    let response = LoginResponse {
        access_token: tokens.get("access_token").cloned().unwrap_or_default(),
        refresh_token: tokens.get("refresh_token").cloned().unwrap_or_default(),
        id_token: tokens.get("id_token").cloned().unwrap_or_default(),
        expires_in: tokens
            .get("expires_in")
            .and_then(|s| s.parse().ok())
            .unwrap_or(3600),
        token_type: "Bearer".to_string(),
    };

    let response_body = serde_json::to_string(&response)
        .map_err(|e| AuthError::InternalError(format!("Failed to serialize response: {}", e)))?;

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(response_body.into())
        .unwrap())
}

/// Handle refresh token request
pub async fn handle_refresh(
    request: Request,
    cognito: &CognitoService,
) -> AuthResult<Response<Body>> {
    info!("Handling refresh token request");

    // Parse request body
    let body = request.body();
    let refresh_req: RefreshRequest = serde_json::from_slice(body.as_ref())
        .map_err(|e| AuthError::InvalidRequest(format!("Invalid JSON: {}", e)))?;

    // Validate input
    if refresh_req.refresh_token.is_empty() {
        return Err(AuthError::InvalidRequest(
            "Refresh token is required".to_string(),
        ));
    }

    // Refresh token with Cognito
    let tokens = cognito.refresh_token(&refresh_req.refresh_token).await?;

    // Build response
    let response = RefreshResponse {
        access_token: tokens.get("access_token").cloned().unwrap_or_default(),
        id_token: tokens.get("id_token").cloned().unwrap_or_default(),
        expires_in: tokens
            .get("expires_in")
            .and_then(|s| s.parse().ok())
            .unwrap_or(3600),
        token_type: "Bearer".to_string(),
    };

    let response_body = serde_json::to_string(&response)
        .map_err(|e| AuthError::InternalError(format!("Failed to serialize response: {}", e)))?;

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(response_body.into())
        .unwrap())
}

/// Handle get user profile request
pub async fn handle_me(
    request: Request,
    jwt_validator: &JwtValidator,
    dynamodb: &DynamoDbService,
) -> AuthResult<Response<Body>> {
    info!("Handling get profile request");

    // Extract and validate JWT token
    let auth_header = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or(AuthError::InvalidToken)?;

    let user_id = jwt_validator.get_user_id_from_token(auth_header).await?;

    // Get user profile from DynamoDB
    let profile = dynamodb.get_user(&user_id).await?;

    // Build response
    let response = MeResponse { profile };

    let response_body = serde_json::to_string(&response)
        .map_err(|e| AuthError::InternalError(format!("Failed to serialize response: {}", e)))?;

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(response_body.into())
        .unwrap())
}
