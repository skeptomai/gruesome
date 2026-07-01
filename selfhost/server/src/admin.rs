//! Admin game-management endpoints (admin-only). Ports the admin Lambda.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::auth::AdminUser;
use crate::db::{now_unix, Game};
use crate::error::{AppError, AppResult};
use crate::games::GameMetadata;
use crate::AppState;

#[derive(Deserialize)]
pub struct CreateGameRequest {
    pub game_id: String,
    pub title: String,
    pub author: String,
    #[serde(default)]
    pub description: String,
    pub category: Option<String>,
    pub year: Option<i64>,
    pub version: i64,
    #[serde(default)]
    pub release: i64,
    #[serde(default)]
    pub serial: String,
    #[serde(default)]
    pub checksum: String,
    #[serde(default)]
    pub file_size: i64,
    pub s3_key: String,
    pub display_order: Option<i64>,
}

#[derive(Deserialize)]
pub struct UpdateGameRequest {
    pub title: String,
    pub author: String,
    #[serde(default)]
    pub description: String,
    pub category: Option<String>,
    pub year: Option<i64>,
    pub display_order: Option<i64>,
}

#[derive(Deserialize)]
pub struct UploadUrlRequest {
    pub filename: String,
}

#[derive(Serialize)]
pub struct AdminListResponse {
    pub games: Vec<GameMetadata>,
    pub total: usize,
}
#[derive(Serialize)]
pub struct UploadUrlResponse {
    pub upload_url: String,
    pub s3_key: String,
    pub expires_in: u64,
}
#[derive(Serialize)]
pub struct MessageResponse {
    pub message: String,
}

pub async fn list_games(
    State(s): State<AppState>,
    _admin: AdminUser,
) -> AppResult<Json<AdminListResponse>> {
    let games: Vec<GameMetadata> =
        s.db.list_all_games()
            .await?
            .into_iter()
            .map(Into::into)
            .collect();
    let total = games.len();
    Ok(Json(AdminListResponse { games, total }))
}

pub async fn create_game(
    State(s): State<AppState>,
    _admin: AdminUser,
    Json(req): Json<CreateGameRequest>,
) -> AppResult<(StatusCode, Json<GameMetadata>)> {
    if req.game_id.trim().is_empty() || req.s3_key.trim().is_empty() {
        return Err(AppError::BadRequest(
            "game_id and s3_key are required".into(),
        ));
    }
    let now = now_unix();
    let game = Game {
        game_id: req.game_id,
        title: req.title,
        author: req.author,
        description: req.description,
        category: req.category,
        year: req.year,
        version: req.version,
        release: req.release,
        serial: req.serial,
        checksum: req.checksum,
        file_size: req.file_size,
        s3_key: req.s3_key,
        display_order: req.display_order,
        archived: 0,
        created_at: now,
        updated_at: now,
    };
    s.db.insert_game(&game).await?;
    Ok((StatusCode::CREATED, Json(game.into())))
}

pub async fn get_game(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path(game_id): Path<String>,
) -> AppResult<Json<GameMetadata>> {
    Ok(Json(s.db.get_game_any(&game_id).await?.into()))
}

pub async fn update_game(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path(game_id): Path<String>,
    Json(req): Json<UpdateGameRequest>,
) -> AppResult<Json<GameMetadata>> {
    s.db.update_game(
        &game_id,
        &req.title,
        &req.author,
        &req.description,
        req.category.as_deref(),
        req.year,
        req.display_order,
        now_unix(),
    )
    .await?;
    Ok(Json(s.db.get_game_any(&game_id).await?.into()))
}

/// Soft delete (archive), matching the AWS version.
pub async fn delete_game(
    State(s): State<AppState>,
    _admin: AdminUser,
    Path(game_id): Path<String>,
) -> AppResult<Json<MessageResponse>> {
    s.db.archive_game(&game_id, now_unix()).await?;
    Ok(Json(MessageResponse {
        message: format!("game '{game_id}' archived"),
    }))
}

/// Presigned URL for uploading a new game file to the games bucket.
pub async fn upload_url(
    State(s): State<AppState>,
    _admin: AdminUser,
    Json(req): Json<UploadUrlRequest>,
) -> AppResult<Json<UploadUrlResponse>> {
    let ext = req.filename.rsplit('.').next().unwrap_or("").to_lowercase();
    if !matches!(ext.as_str(), "z3" | "z4" | "z5" | "z8") {
        return Err(AppError::BadRequest(
            "filename must be a Z-Machine game (.z3/.z4/.z5/.z8)".into(),
        ));
    }
    let s3_key = format!("uploads/{}.{}", uuid::Uuid::new_v4(), ext);
    let url =
        s.s3.presign_put(&s.cfg.games_bucket, &s3_key, s.cfg.presign_ttl)
            .await?;
    Ok(Json(UploadUrlResponse {
        upload_url: url,
        s3_key,
        expires_in: s.cfg.presign_ttl.as_secs(),
    }))
}
