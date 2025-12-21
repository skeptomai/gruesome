use lambda_http::{run, service_fn, Body, Error, Request, Response};
use tracing::info;

mod dynamodb_service;
mod error;
mod handlers;
mod jwt_auth;
mod models;
mod s3_service;

use dynamodb_service::DynamoDbService;
use error::AdminError;
use jwt_auth::JwtValidator;
use s3_service::S3Service;

/// Main Lambda function handler with routing
async fn function_handler(
    event: Request,
    jwt_validator: &JwtValidator,
    dynamodb: &DynamoDbService,
    s3: &S3Service,
) -> Result<Response<Body>, Error> {
    info!(
        "Handling request: {} {}",
        event.method(),
        event.uri().path()
    );

    // Route based on path and method
    let path = event.uri().path().to_string();
    let method = event.method().as_str();

    let response = match (method, path.as_str()) {
        ("GET", "/api/admin/games") => {
            handlers::handle_list_games(event, jwt_validator, dynamodb).await
        }
        ("POST", "/api/admin/games") => {
            handlers::handle_create_game(event, jwt_validator, dynamodb).await
        }
        ("POST", "/api/admin/games/upload-url") => {
            handlers::handle_upload_url(event, jwt_validator, dynamodb, s3).await
        }
        ("GET", path) if path.starts_with("/api/admin/games/") => {
            let game_id = path.trim_start_matches("/api/admin/games/");
            handlers::handle_get_game(event, jwt_validator, dynamodb, game_id).await
        }
        ("PUT", path) if path.starts_with("/api/admin/games/") => {
            let game_id = path.trim_start_matches("/api/admin/games/");
            handlers::handle_update_game(event, jwt_validator, dynamodb, game_id).await
        }
        ("DELETE", path) if path.starts_with("/api/admin/games/") => {
            let game_id = path.trim_start_matches("/api/admin/games/");
            handlers::handle_delete_game(event, jwt_validator, dynamodb, game_id).await
        }
        ("GET", "/health") => {
            // Health check endpoint
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(r#"{"status":"healthy"}"#.into())
                .unwrap())
        }
        _ => {
            // Route not found
            Err(AdminError::InvalidRequest(format!(
                "Route not found: {} {}",
                method, path
            )))
        }
    };

    // Convert AdminError to Response if needed
    match response {
        Ok(resp) => Ok(resp),
        Err(err) => Ok(err.to_response()),
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .without_time()
        .init();

    info!("Starting Admin Lambda function");

    // Load AWS configuration
    let config = aws_config::load_from_env().await;

    // Get environment variables
    let table_name = std::env::var("TABLE_NAME").expect("TABLE_NAME must be set");
    let bucket_name = std::env::var("BUCKET_NAME").expect("BUCKET_NAME must be set");
    let user_pool_id = std::env::var("USER_POOL_ID").expect("USER_POOL_ID must be set");
    let aws_region = std::env::var("AWS_REGION").unwrap_or_else(|_| "us-west-1".to_string());

    info!("Configuration loaded:");
    info!("  Table: {}", table_name);
    info!("  Bucket: {}", bucket_name);
    info!("  User Pool: {}", user_pool_id);
    info!("  Region: {}", aws_region);

    // Initialize AWS clients
    let dynamodb_client = aws_sdk_dynamodb::Client::new(&config);
    let s3_client = aws_sdk_s3::Client::new(&config);

    // Initialize services
    let dynamodb = DynamoDbService::new(dynamodb_client, table_name);
    let s3 = S3Service::new(s3_client, bucket_name);
    let jwt_validator = JwtValidator::new(aws_region, user_pool_id);

    info!("Services initialized successfully");

    // Run Lambda runtime
    run(service_fn(|event: Request| async {
        function_handler(event, &jwt_validator, &dynamodb, &s3).await
    }))
    .await
}
