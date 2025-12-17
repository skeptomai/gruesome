mod error;
mod models;
mod game_service;
mod save_service;
mod handlers;
mod jwt_auth;

use lambda_http::{run, service_fn, Request, Response, Body, Error};
use tracing_subscriber;

use crate::error::GameError;
use crate::game_service::GameService;
use crate::save_service::SaveService;
use crate::jwt_auth::JwtValidator;

async fn function_handler(
    event: Request,
    game_service: &GameService,
    save_service: &SaveService,
    jwt_validator: &JwtValidator,
) -> Result<Response<Body>, Error> {
    let path = event.uri().path();
    let method = event.method().as_str();

    let response = match (method, path) {
        // Game endpoints
        ("GET", "/api/games") =>
            handlers::handle_list_games(game_service).await,
        ("GET", path) if path.starts_with("/api/games/") && path.ends_with("/file") =>
            handlers::handle_get_game_file(event, game_service).await,
        ("GET", path) if path.starts_with("/api/games/") =>
            handlers::handle_get_game(event, game_service).await,

        // Save endpoints
        ("GET", "/api/saves") =>
            handlers::handle_list_saves(event, save_service, jwt_validator).await,
        ("GET", path) if path.starts_with("/api/saves/") && path.matches('/').count() == 3 =>
            handlers::handle_list_game_saves(event, save_service, jwt_validator).await,
        ("GET", path) if path.starts_with("/api/saves/") && path.matches('/').count() == 4 =>
            handlers::handle_get_save(event, save_service, jwt_validator).await,
        ("POST", path) if path.starts_with("/api/saves/") =>
            handlers::handle_create_save(event, save_service, jwt_validator).await,
        ("DELETE", path) if path.starts_with("/api/saves/") =>
            handlers::handle_delete_save(event, save_service, jwt_validator).await,

        // Health check
        ("GET", "/health") => {
            let body = serde_json::json!({"status": "healthy"}).to_string();
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(body.into())
                .unwrap())
        }

        _ => Err(GameError::InvalidRequest(format!("Route not found: {} {}", method, path))),
    };

    match response {
        Ok(resp) => Ok(resp),
        Err(err) => Ok(err.to_response()),
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .without_time()
        .init();

    let config = aws_config::load_from_env().await;
    let region = config.region().map(|r| r.to_string()).unwrap_or_else(|| "us-west-1".to_string());

    // Environment variables
    let table_name = std::env::var("TABLE_NAME")
        .expect("TABLE_NAME environment variable must be set");
    let games_bucket = std::env::var("GAMES_BUCKET")
        .expect("GAMES_BUCKET environment variable must be set");
    let saves_bucket = std::env::var("SAVES_BUCKET")
        .expect("SAVES_BUCKET environment variable must be set");
    let user_pool_id = std::env::var("USER_POOL_ID")
        .expect("USER_POOL_ID environment variable must be set");

    // AWS clients
    let dynamodb_client = aws_sdk_dynamodb::Client::new(&config);
    let s3_client = aws_sdk_s3::Client::new(&config);

    // Services
    let game_service = GameService::new(
        dynamodb_client.clone(),
        s3_client.clone(),
        table_name.clone(),
        games_bucket,
    );

    let save_service = SaveService::new(
        dynamodb_client,
        s3_client,
        table_name,
        saves_bucket,
    );

    let jwt_validator = JwtValidator::new(region, user_pool_id);

    run(service_fn(|event: Request| async {
        function_handler(event, &game_service, &save_service, &jwt_validator).await
    })).await
}
