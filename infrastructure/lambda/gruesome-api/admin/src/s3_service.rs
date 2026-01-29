use aws_sdk_s3::{presigning::PresigningConfig, Client};
use std::time::Duration;
use tracing::{error, info};
use uuid::Uuid;

use crate::error::{AdminError, AdminResult};

pub struct S3Service {
    client: Client,
    bucket_name: String,
}

impl S3Service {
    pub fn new(client: Client, bucket_name: String) -> Self {
        Self {
            client,
            bucket_name,
        }
    }

    /// Generate presigned URL for uploading a game file
    pub async fn generate_upload_url(&self, filename: &str) -> AdminResult<(String, String)> {
        info!("Generating presigned upload URL for: {}", filename);

        // Generate unique S3 key using UUID + original filename
        let file_ext = filename.rsplit('.').next().unwrap_or("z3");

        let unique_id = Uuid::new_v4();
        let s3_key = format!("uploads/{}.{}", unique_id, file_ext);

        // Configure presigning (5 minute expiry)
        let expires_in = Duration::from_secs(300);
        let presigning_config = PresigningConfig::expires_in(expires_in).map_err(|e| {
            error!("Failed to create presigning config: {:?}", e);
            AdminError::S3Error(format!("Presigning configuration error: {}", e))
        })?;

        // Generate presigned PUT URL
        let presigned_request = self
            .client
            .put_object()
            .bucket(&self.bucket_name)
            .key(&s3_key)
            .content_type("application/octet-stream")
            .presigned(presigning_config)
            .await
            .map_err(|e| {
                error!("Failed to generate presigned URL: {:?}", e);
                AdminError::S3Error(format!("Failed to generate upload URL: {}", e))
            })?;

        let upload_url = presigned_request.uri().to_string();

        info!("Generated presigned URL for S3 key: {}", s3_key);
        Ok((upload_url, s3_key))
    }
}
