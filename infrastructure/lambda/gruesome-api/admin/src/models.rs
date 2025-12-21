use serde::{Deserialize, Serialize};

// ============================================================================
// Game Metadata Models
// ============================================================================

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameMetadata {
    pub game_id: String,
    pub title: String,
    pub author: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<i32>,
    pub version: i32,
    pub release: i32,
    pub serial: String,
    pub checksum: String,
    pub file_size: i64,
    pub s3_key: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub archived: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_order: Option<i32>,
}

// ============================================================================
// List Games
// ============================================================================

#[derive(Serialize)]
pub struct ListGamesResponse {
    pub games: Vec<GameMetadata>,
    pub total: usize,
}

// ============================================================================
// Create Game
// ============================================================================

#[derive(Deserialize, Debug)]
pub struct CreateGameRequest {
    pub game_id: String,
    pub title: String,
    pub author: String,
    pub description: String,
    pub category: Option<String>,
    pub year: Option<i32>,
    pub version: i32,
    pub release: i32,
    pub serial: String,
    pub checksum: String,
    pub file_size: i64,
    pub s3_key: String,
    pub display_order: Option<i32>,
}

// ============================================================================
// Update Game
// ============================================================================

#[derive(Deserialize, Debug)]
pub struct UpdateGameRequest {
    pub title: String,
    pub author: String,
    pub description: String,
    pub category: Option<String>,
    pub year: Option<i32>,
    pub display_order: Option<i32>,
}

// ============================================================================
// Upload URL
// ============================================================================

#[derive(Deserialize, Debug)]
pub struct UploadUrlRequest {
    pub filename: String,
}

#[derive(Serialize)]
pub struct UploadUrlResponse {
    pub upload_url: String,
    pub s3_key: String,
    pub expires_in: u64,
}

// ============================================================================
// Generic Success Response
// ============================================================================

#[derive(Serialize)]
pub struct SuccessResponse {
    pub message: String,
}
