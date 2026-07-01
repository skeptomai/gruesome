//! Runtime configuration, from environment variables.
//!
//! Replaces the AWS Lambda env surface (`TABLE_NAME`, `USER_POOL_ID`,
//! `GAMES_BUCKET`, `AWS_REGION`, …) with self-hosted equivalents: a SQLite
//! path, a MinIO endpoint + credentials, a JWT signing secret, and bucket names.

use std::time::Duration;

#[derive(Clone, Debug)]
pub struct Config {
    /// `host:port` the HTTP server binds to (default `0.0.0.0:8080`).
    pub bind_addr: String,
    /// SQLite database URL, e.g. `sqlite:///data/gruesome.db` (default `sqlite://gruesome.db`).
    pub database_url: String,
    /// Directory of static frontend files to serve at `/` (empty = don't serve static).
    pub frontend_dir: String,

    /// HS256 secret used to sign and verify JWTs. MUST be set in production.
    pub jwt_secret: String,
    /// Access-token lifetime.
    pub access_token_ttl: Duration,
    /// Refresh-token lifetime.
    pub refresh_token_ttl: Duration,

    /// S3-compatible endpoint (MinIO), e.g. `http://minio.gruesome.svc:9000`.
    pub s3_endpoint: String,
    pub s3_region: String,
    pub s3_access_key: String,
    pub s3_secret_key: String,
    pub games_bucket: String,
    pub saves_bucket: String,
    /// Presigned-URL lifetime (matches the AWS version's 300s).
    pub presign_ttl: Duration,

    /// Allowed CORS origin for the browser SPA (e.g. `https://gruesome.home`),
    /// or `*` for any (dev).
    pub cors_origin: String,
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn env_secs(key: &str, default: u64) -> Duration {
    Duration::from_secs(
        std::env::var(key)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(default),
    )
}

impl Config {
    pub fn from_env() -> Self {
        Config {
            bind_addr: env_or("BIND_ADDR", "0.0.0.0:8080"),
            database_url: env_or("DATABASE_URL", "sqlite://gruesome.db"),
            frontend_dir: env_or("FRONTEND_DIR", ""),

            jwt_secret: env_or("JWT_SECRET", "dev-insecure-change-me"),
            access_token_ttl: env_secs("ACCESS_TOKEN_TTL_SECS", 3600), // 1h
            refresh_token_ttl: env_secs("REFRESH_TOKEN_TTL_SECS", 60 * 60 * 24 * 30), // 30d

            s3_endpoint: env_or("S3_ENDPOINT", "http://localhost:9000"),
            s3_region: env_or("S3_REGION", "us-east-1"),
            s3_access_key: env_or("S3_ACCESS_KEY", "minioadmin"),
            s3_secret_key: env_or("S3_SECRET_KEY", "minioadmin"),
            games_bucket: env_or("GAMES_BUCKET", "gruesome-games"),
            saves_bucket: env_or("SAVES_BUCKET", "gruesome-saves"),
            presign_ttl: env_secs("PRESIGN_TTL_SECS", 300),

            cors_origin: env_or("CORS_ORIGIN", "*"),
        }
    }

    /// True if the JWT secret is still the built-in dev default (used to warn loudly).
    pub fn jwt_secret_is_default(&self) -> bool {
        self.jwt_secret == "dev-insecure-change-me"
    }
}
