//! Object storage via the S3-compatible SDK, pointed at MinIO.
//!
//! Same presigned-URL model as the AWS version (browser uploads/downloads blobs
//! directly), but the client is built with a custom endpoint + path-style
//! addressing so it targets MinIO instead of AWS S3. SigV4 signing is identical,
//! so MinIO accepts the presigned URLs unchanged.
//!
//! The client is constructed entirely from static config — no `aws_config`
//! default provider chain (which would try IMDS/env/profile lookups and can hang
//! when there's no AWS environment). Retries are disabled and operations are
//! short-timed so a missing MinIO fails fast instead of blocking startup.

use std::time::Duration;

use aws_sdk_s3::config::retry::RetryConfig;
use aws_sdk_s3::config::timeout::TimeoutConfig;
use aws_sdk_s3::config::{BehaviorVersion, Credentials, Region};
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::Client;

use crate::config::Config;
use crate::error::{AppError, AppResult};

#[derive(Clone)]
pub struct S3Store {
    client: Client,
}

impl S3Store {
    pub fn new(cfg: &Config) -> Self {
        let creds = Credentials::new(
            cfg.s3_access_key.clone(),
            cfg.s3_secret_key.clone(),
            None,
            None,
            "static",
        );
        let conf = aws_sdk_s3::Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new(cfg.s3_region.clone()))
            .endpoint_url(cfg.s3_endpoint.clone())
            .credentials_provider(creds)
            .force_path_style(true) // MinIO needs path-style bucket addressing
            .retry_config(RetryConfig::disabled())
            .timeout_config(
                TimeoutConfig::builder()
                    .operation_timeout(Duration::from_secs(10))
                    .connect_timeout(Duration::from_secs(3))
                    .build(),
            )
            .build();
        S3Store {
            client: Client::from_conf(conf),
        }
    }

    /// Create a bucket if absent (idempotent, best-effort — logged, never fatal).
    pub async fn ensure_bucket(&self, bucket: &str) {
        if self
            .client
            .head_bucket()
            .bucket(bucket)
            .send()
            .await
            .is_ok()
        {
            return;
        }
        match self.client.create_bucket().bucket(bucket).send().await {
            Ok(_) => tracing::info!("created bucket '{bucket}'"),
            Err(e) => tracing::warn!("could not ensure bucket '{bucket}': {e}"),
        }
    }

    pub async fn presign_get(&self, bucket: &str, key: &str, ttl: Duration) -> AppResult<String> {
        let cfg =
            PresigningConfig::expires_in(ttl).map_err(|e| AppError::Internal(e.to_string()))?;
        let req = self
            .client
            .get_object()
            .bucket(bucket)
            .key(key)
            .presigned(cfg)
            .await
            .map_err(|e| AppError::Internal(format!("presign get: {e}")))?;
        Ok(req.uri().to_string())
    }

    pub async fn presign_put(&self, bucket: &str, key: &str, ttl: Duration) -> AppResult<String> {
        let cfg =
            PresigningConfig::expires_in(ttl).map_err(|e| AppError::Internal(e.to_string()))?;
        let req = self
            .client
            .put_object()
            .bucket(bucket)
            .key(key)
            .content_type("application/octet-stream")
            .presigned(cfg)
            .await
            .map_err(|e| AppError::Internal(format!("presign put: {e}")))?;
        Ok(req.uri().to_string())
    }

    pub async fn delete(&self, bucket: &str, key: &str) -> AppResult<()> {
        self.client
            .delete_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("s3 delete: {e}")))?;
        Ok(())
    }
}
