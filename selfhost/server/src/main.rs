//! Gruesome self-hosted platform server.
//!
//! One long-running HTTP service that replaces the AWS serverless backend
//! (3 Lambdas + API Gateway + Cognito + DynamoDB + S3) with: axum + SQLite +
//! MinIO (S3-compatible) + self-issued JWTs. The Z-Machine interpreter itself
//! runs client-side as WASM — this server is pure storage + auth.

mod admin;
mod auth;
mod config;
mod db;
mod error;
mod games;
mod s3;
mod saves;

use std::sync::Arc;

use axum::http::{header, HeaderValue, Method};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::json;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

use config::Config;
use db::Db;

/// Shared application state (cloned per request; all fields are cheap to clone).
#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub s3: s3::S3Store,
    pub cfg: Arc<Config>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,gruesome_platform_server=debug".into()),
        )
        .init();

    let cfg = Config::from_env();
    if cfg.jwt_secret_is_default() {
        tracing::warn!(
            "JWT_SECRET is the built-in dev default — set a strong secret in production!"
        );
    }

    let db = Db::connect(&cfg.database_url).await?;
    tracing::info!("connected to {}", cfg.database_url);

    let s3 = s3::S3Store::new(&cfg);
    // Best-effort bucket bootstrap, time-boxed so a down MinIO never blocks
    // startup (presigning is a local op and works regardless).
    for bucket in [&cfg.games_bucket, &cfg.saves_bucket] {
        let _ =
            tokio::time::timeout(std::time::Duration::from_secs(5), s3.ensure_bucket(bucket)).await;
    }

    let state = AppState {
        db,
        s3,
        cfg: Arc::new(cfg.clone()),
    };

    let mut router = Router::new()
        .route("/health", get(health))
        .route("/api/auth/signup", post(auth::signup))
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/refresh", post(auth::refresh))
        .route("/api/auth/forgot-password", post(auth::forgot_password))
        .route(
            "/api/auth/confirm-forgot-password",
            post(auth::confirm_forgot_password),
        )
        .route("/api/auth/me", get(auth::me))
        .route("/api/auth/change-password", post(auth::change_password))
        // Games (public)
        .route("/api/games", get(games::list_games))
        .route("/api/games/:game_id", get(games::get_game))
        .route("/api/games/:game_id/file", get(games::get_game_file))
        // Saves (auth-required)
        .route("/api/saves", get(saves::list_saves))
        .route("/api/saves/:game_id", get(saves::list_saves_for_game))
        .route(
            "/api/saves/:game_id/:save_name",
            get(saves::get_save_download)
                .post(saves::create_save)
                .delete(saves::delete_save),
        )
        // Admin (admin-only). Static `upload-url` is registered before the
        // `:game_id` param route so it isn't captured as an id.
        .route(
            "/api/admin/games",
            get(admin::list_games).post(admin::create_game),
        )
        .route("/api/admin/games/upload-url", post(admin::upload_url))
        .route(
            "/api/admin/games/:game_id",
            get(admin::get_game)
                .put(admin::update_game)
                .delete(admin::delete_game),
        );

    // Optionally serve the static SPA (index.html, app.js, wasm) as a fallback.
    if !cfg.frontend_dir.is_empty() {
        router = router.fallback_service(ServeDir::new(&cfg.frontend_dir));
    }

    let app = router
        .layer(build_cors(&cfg.cors_origin))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&cfg.bind_addr).await?;
    tracing::info!("listening on http://{}", cfg.bind_addr);
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({ "status": "healthy" }))
}

fn build_cors(origin: &str) -> CorsLayer {
    let base = CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE]);
    match (origin, origin.parse::<HeaderValue>()) {
        ("*", _) => base.allow_origin(Any),
        (_, Ok(hv)) => base.allow_origin(hv),
        (_, Err(_)) => {
            tracing::warn!("invalid CORS_ORIGIN '{origin}', falling back to any-origin");
            base.allow_origin(Any)
        }
    }
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutting down");
}
