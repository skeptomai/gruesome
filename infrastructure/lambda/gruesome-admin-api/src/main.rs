use lambda_http::{run, service_fn, Body, Error, Request, Response};
use tracing::info;

mod auth;
mod error;
mod games;
mod metadata;
mod upload;
mod validation;

use error::ApiError;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .without_time()
        .init();

    info!("Starting gruesome-admin-api Lambda");

    run(service_fn(function_handler)).await
}

async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    let method = event.method().to_string();
    let path = event.uri().path().to_string();

    info!("Request: {} {}", method, path);

    // Route requests to appropriate handlers
    let result = match (method.as_str(), path.as_str()) {
        ("POST", "/api/admin/games/upload-url") => upload::handle_upload_url(event).await,
        ("POST", "/api/admin/games") => games::handle_create_game(event).await,
        ("GET", "/api/admin/games") => games::handle_list_games(event).await,
        ("GET", p) if p.starts_with("/api/admin/games/") => {
            let game_id = p.trim_start_matches("/api/admin/games/");
            games::handle_get_game(event, game_id).await
        }
        ("PUT", p) if p.starts_with("/api/admin/games/") => {
            let game_id = p.trim_start_matches("/api/admin/games/");
            games::handle_update_game(event, game_id).await
        }
        ("DELETE", p) if p.starts_with("/api/admin/games/") => {
            let game_id = p.trim_start_matches("/api/admin/games/");
            games::handle_delete_game(event, game_id).await
        }
        _ => Err(ApiError::NotFound),
    };

    // Convert result to HTTP response
    match result {
        Ok(response) => Ok(response),
        Err(e) => Ok(e.into_response()),
    }
}
