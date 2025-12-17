use lambda_http::{run, service_fn, Body, Error, Request, Response};
use tracing::info;

mod cognito;
mod dynamodb_service;
mod error;
mod handlers;
mod jwt_auth;
mod models;

use cognito::CognitoService;
use dynamodb_service::DynamoDbService;
use error::AuthError;
use jwt_auth::JwtValidator;

/// Main Lambda function handler with routing
async fn function_handler(
    event: Request,
    cognito: &CognitoService,
    dynamodb: &DynamoDbService,
    jwt_validator: &JwtValidator,
) -> Result<Response<Body>, Error> {
    info!(
        "Handling request: {} {}",
        event.method(),
        event.uri().path()
    );

    // Route based on path and method
    let path = event.uri().path();
    let method = event.method().as_str();

    let response = match (method, path) {
        ("POST", "/api/auth/signup") => {
            handlers::handle_signup(event, cognito, dynamodb).await
        }
        ("POST", "/api/auth/login") => {
            handlers::handle_login(event, cognito).await
        }
        ("POST", "/api/auth/refresh") => {
            handlers::handle_refresh(event, cognito).await
        }
        ("GET", "/api/auth/me") => {
            handlers::handle_me(event, jwt_validator, dynamodb).await
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
            Err(AuthError::InvalidRequest(format!(
                "Route not found: {} {}",
                method, path
            )))
        }
    };

    // Convert AuthError to Response if needed
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

    info!("Starting Lambda function");

    // Load AWS configuration
    let config = aws_config::load_from_env().await;

    // Get environment variables
    let table_name = std::env::var("TABLE_NAME").expect("TABLE_NAME must be set");
    let user_pool_id = std::env::var("USER_POOL_ID").expect("USER_POOL_ID must be set");
    let user_pool_client_id =
        std::env::var("USER_POOL_CLIENT_ID").expect("USER_POOL_CLIENT_ID must be set");
    let aws_region = std::env::var("AWS_REGION").unwrap_or_else(|_| "us-west-1".to_string());

    info!("Configuration loaded:");
    info!("  Table: {}", table_name);
    info!("  User Pool: {}", user_pool_id);
    info!("  Region: {}", aws_region);

    // Initialize AWS clients
    let cognito_client = aws_sdk_cognitoidentityprovider::Client::new(&config);
    let dynamodb_client = aws_sdk_dynamodb::Client::new(&config);

    // Initialize services
    let cognito = CognitoService::new(cognito_client, user_pool_id.clone(), user_pool_client_id);
    let dynamodb = DynamoDbService::new(dynamodb_client, table_name);
    let jwt_validator = JwtValidator::new(aws_region, user_pool_id);

    info!("Services initialized successfully");

    // Run Lambda runtime
    run(service_fn(|event: Request| async {
        function_handler(event, &cognito, &dynamodb, &jwt_validator).await
    }))
    .await
}
