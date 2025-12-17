use serde::{Deserialize, Serialize};

// ========== Game Models ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameMetadata {
    pub game_id: String,
    pub title: String,
    pub author: String,
    pub description: String,
    pub version: u8,          // Z-Machine version
    pub file_size: u64,       // File size in bytes
    pub s3_key: String,       // S3 object key
    pub created_at: i64,      // Unix timestamp
}

#[derive(Serialize)]
pub struct ListGamesResponse {
    pub games: Vec<GameMetadata>,
}

#[derive(Serialize)]
pub struct GetGameResponse {
    #[serde(flatten)]
    pub game: GameMetadata,
}

#[derive(Serialize)]
pub struct GetGameFileResponse {
    pub download_url: String,
    pub expires_in: u64,  // Seconds until URL expires
}

// ========== Save Models ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveMetadata {
    pub user_id: String,
    pub game_id: String,
    pub save_name: String,
    pub s3_key: String,
    pub file_size: u64,
    pub created_at: i64,
    pub last_updated: i64,
}

#[derive(Serialize)]
pub struct ListSavesResponse {
    pub saves: Vec<SaveMetadata>,
}

#[derive(Serialize)]
pub struct GetSaveDownloadResponse {
    pub download_url: String,
    pub expires_in: u64,
}

#[derive(Deserialize)]
pub struct CreateSaveRequest {
    pub file_size: Option<u64>,
}

#[derive(Serialize)]
pub struct CreateSaveResponse {
    pub upload_url: String,
    pub expires_in: u64,
}

#[derive(Serialize)]
pub struct DeleteSaveResponse {
    pub deleted: bool,
}

// ========== Error Response ==========

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}
