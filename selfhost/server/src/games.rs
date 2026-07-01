//! Public game endpoints (list / metadata / download-URL). Ports the game Lambda.

use axum::extract::{Path, State};
use axum::Json;
use serde::Serialize;

use crate::db::Game;
use crate::error::AppResult;
use crate::AppState;

/// Player-facing projection of a game (matches the frontend's `GameMetadata`).
#[derive(Serialize)]
pub struct GameMetadata {
    pub game_id: String,
    pub title: String,
    pub author: String,
    pub description: String,
    pub version: u8,
    pub file_size: u64,
    pub s3_key: String,
    pub created_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_order: Option<i32>,
}

impl From<Game> for GameMetadata {
    fn from(g: Game) -> Self {
        GameMetadata {
            game_id: g.game_id,
            title: g.title,
            author: g.author,
            description: g.description,
            version: g.version.clamp(0, 255) as u8,
            file_size: g.file_size.max(0) as u64,
            s3_key: g.s3_key,
            created_at: g.created_at,
            display_order: g.display_order.map(|d| d as i32),
        }
    }
}

#[derive(Serialize)]
pub struct ListGamesResponse {
    pub games: Vec<GameMetadata>,
}

#[derive(Serialize)]
pub struct FileUrlResponse {
    pub download_url: String,
    pub expires_in: u64,
}

pub async fn list_games(State(s): State<AppState>) -> AppResult<Json<ListGamesResponse>> {
    let games =
        s.db.list_active_games()
            .await?
            .into_iter()
            .map(Into::into)
            .collect();
    Ok(Json(ListGamesResponse { games }))
}

pub async fn get_game(
    State(s): State<AppState>,
    Path(game_id): Path<String>,
) -> AppResult<Json<GameMetadata>> {
    Ok(Json(s.db.get_active_game(&game_id).await?.into()))
}

pub async fn get_game_file(
    State(s): State<AppState>,
    Path(game_id): Path<String>,
) -> AppResult<Json<FileUrlResponse>> {
    let game = s.db.get_active_game(&game_id).await?;
    let url =
        s.s3.presign_get(&s.cfg.games_bucket, &game.s3_key, s.cfg.presign_ttl)
            .await?;
    Ok(Json(FileUrlResponse {
        download_url: url,
        expires_in: s.cfg.presign_ttl.as_secs(),
    }))
}
