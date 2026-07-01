//! User save endpoints (auth-required). Ports the game Lambda's save routes.
//! Save blobs live in the saves bucket under `{user_id}/{game_id}/{save_name}.sav`.

use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::auth::AuthUser;
use crate::db::{now_unix, Save};
use crate::error::AppResult;
use crate::AppState;

#[derive(Serialize)]
pub struct SaveMetadata {
    pub user_id: String,
    pub game_id: String,
    pub save_name: String,
    pub s3_key: String,
    pub file_size: u64,
    pub created_at: i64,
    pub last_updated: i64,
}

impl From<Save> for SaveMetadata {
    fn from(s: Save) -> Self {
        SaveMetadata {
            user_id: s.user_id,
            game_id: s.game_id,
            save_name: s.save_name,
            s3_key: s.s3_key,
            file_size: s.file_size.max(0) as u64,
            created_at: s.created_at,
            last_updated: s.last_updated,
        }
    }
}

#[derive(Serialize)]
pub struct ListSavesResponse {
    pub saves: Vec<SaveMetadata>,
}
#[derive(Serialize)]
pub struct DownloadUrlResponse {
    pub download_url: String,
    pub expires_in: u64,
}
#[derive(Deserialize, Default)]
pub struct CreateSaveRequest {
    pub file_size: Option<u64>,
}
#[derive(Serialize)]
pub struct UploadUrlResponse {
    pub upload_url: String,
    pub expires_in: u64,
}
#[derive(Serialize)]
pub struct DeleteSaveResponse {
    pub deleted: bool,
}

fn save_key(user_id: &str, game_id: &str, save_name: &str) -> String {
    format!("{user_id}/{game_id}/{save_name}.sav")
}

pub async fn list_saves(
    State(s): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<ListSavesResponse>> {
    let saves =
        s.db.list_saves(&user.user_id)
            .await?
            .into_iter()
            .map(Into::into)
            .collect();
    Ok(Json(ListSavesResponse { saves }))
}

pub async fn list_saves_for_game(
    State(s): State<AppState>,
    user: AuthUser,
    Path(game_id): Path<String>,
) -> AppResult<Json<ListSavesResponse>> {
    let saves =
        s.db.list_saves_for_game(&user.user_id, &game_id)
            .await?
            .into_iter()
            .map(Into::into)
            .collect();
    Ok(Json(ListSavesResponse { saves }))
}

pub async fn get_save_download(
    State(s): State<AppState>,
    user: AuthUser,
    Path((game_id, save_name)): Path<(String, String)>,
) -> AppResult<Json<DownloadUrlResponse>> {
    let save = s.db.get_save(&user.user_id, &game_id, &save_name).await?;
    let url =
        s.s3.presign_get(&s.cfg.saves_bucket, &save.s3_key, s.cfg.presign_ttl)
            .await?;
    Ok(Json(DownloadUrlResponse {
        download_url: url,
        expires_in: s.cfg.presign_ttl.as_secs(),
    }))
}

pub async fn create_save(
    State(s): State<AppState>,
    user: AuthUser,
    Path((game_id, save_name)): Path<(String, String)>,
    body: axum::body::Bytes,
) -> AppResult<Json<UploadUrlResponse>> {
    // Body is optional (frontend may POST without one); tolerate empty/missing.
    let req: CreateSaveRequest = if body.is_empty() {
        CreateSaveRequest::default()
    } else {
        serde_json::from_slice(&body).unwrap_or_default()
    };
    let key = save_key(&user.user_id, &game_id, &save_name);
    let now = now_unix();
    // Register/refresh the metadata; the browser then PUTs the blob to the URL.
    let save = Save {
        user_id: user.user_id.clone(),
        game_id,
        save_name,
        s3_key: key.clone(),
        file_size: req.file_size.unwrap_or(0) as i64,
        created_at: now,
        last_updated: now,
    };
    s.db.upsert_save(&save).await?;
    let url =
        s.s3.presign_put(&s.cfg.saves_bucket, &key, s.cfg.presign_ttl)
            .await?;
    Ok(Json(UploadUrlResponse {
        upload_url: url,
        expires_in: s.cfg.presign_ttl.as_secs(),
    }))
}

pub async fn delete_save(
    State(s): State<AppState>,
    user: AuthUser,
    Path((game_id, save_name)): Path<(String, String)>,
) -> AppResult<Json<DeleteSaveResponse>> {
    let save = s.db.get_save(&user.user_id, &game_id, &save_name).await?;
    s.s3.delete(&s.cfg.saves_bucket, &save.s3_key).await?;
    s.db.delete_save(&user.user_id, &game_id, &save_name)
        .await?;
    Ok(Json(DeleteSaveResponse { deleted: true }))
}
