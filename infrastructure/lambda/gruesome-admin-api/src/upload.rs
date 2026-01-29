use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::Client as S3Client;
use lambda_http::{Body, Request, Response};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::info;

use crate::auth::require_admin;
use crate::error::ApiError;
use crate::validation::sanitize_filename;

#[derive(Deserialize)]
struct UploadUrlRequest {
    filename: String,
}

#[derive(Serialize)]
struct UploadUrlResponse {
    upload_url: String,
    s3_key: String,
    expires_in: u64,
}

/// Handle POST /api/admin/games/upload-url
///
/// Generates a presigned S3 URL for uploading a game file
pub async fn handle_upload_url(event: Request) -> Result<Response<Body>, ApiError> {
    // Initialize AWS clients
    let config = aws_config::load_from_env().await;
    let dynamodb = aws_sdk_dynamodb::Client::new(&config);
    let s3_client = S3Client::new(&config);

    // Get table name and bucket name from environment
    let table_name = std::env::var("TABLE_NAME").unwrap_or_else(|_| "gruesome-platform".to_string());
    let bucket_name = std::env::var("GAMES_BUCKET")
        .unwrap_or_else(|_| "gruesome-games".to_string());

    // Verify admin role
    let user_id = require_admin(&event, &dynamodb, &table_name).await?;
    info!("Admin user {} requesting presigned upload URL", user_id);

    // Parse request body
    let body = event.body();
    let request: UploadUrlRequest = serde_json::from_slice(body)?;

    // Sanitize and validate filename
    let sanitized_filename = sanitize_filename(&request.filename)?;
    let s3_key = format!("games/{}", sanitized_filename);

    info!(
        "Generating presigned URL for S3 key: {} (bucket: {})",
        s3_key, bucket_name
    );

    // Generate presigned URL (5 minutes expiry)
    let expires_in = Duration::from_secs(300); // 5 minutes

    let presigned_request = s3_client
        .put_object()
        .bucket(&bucket_name)
        .key(&s3_key)
        .content_type("application/octet-stream")
        .presigned(PresigningConfig::expires_in(expires_in).map_err(|e| {
            ApiError::InternalError(format!("Failed to create presigning config: {}", e))
        })?)
        .await
        .map_err(|e| ApiError::S3Error(format!("Failed to generate presigned URL: {:?}", e)))?;

    let upload_url = presigned_request.uri().to_string();

    info!("Generated presigned URL for user {}", user_id);

    // Return response
    let response = UploadUrlResponse {
        upload_url,
        s3_key,
        expires_in: 300,
    };

    let body = serde_json::to_string(&response)?;

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(Body::Text(body))
        .unwrap())
}
