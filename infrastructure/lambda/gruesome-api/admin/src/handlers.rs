use lambda_http::{Body, Request, Response};
use tracing::info;

use crate::dynamodb_service::DynamoDbService;
use crate::error::{AdminError, AdminResult};
use crate::jwt_auth::JwtValidator;
use crate::models::*;
use crate::s3_service::S3Service;

/// Verify admin authorization for all admin endpoints
async fn verify_admin(
    request: &Request,
    jwt_validator: &JwtValidator,
    dynamodb: &DynamoDbService,
) -> AdminResult<String> {
    // Extract Authorization header
    let auth_header = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or(AdminError::Unauthorized(
            "Missing authorization header".to_string(),
        ))?;

    // Validate JWT and get user ID
    let user_id = jwt_validator.get_user_id_from_token(auth_header).await?;

    // Check if user has admin role
    let is_admin = dynamodb.check_admin_role(&user_id).await?;
    if !is_admin {
        return Err(AdminError::Forbidden("Admin access required".to_string()));
    }

    Ok(user_id)
}

/// Handle GET /api/admin/games - List all games
pub async fn handle_list_games(
    request: Request,
    jwt_validator: &JwtValidator,
    dynamodb: &DynamoDbService,
) -> AdminResult<Response<Body>> {
    info!("Handling list games request");

    // Verify admin authorization
    verify_admin(&request, jwt_validator, dynamodb).await?;

    // Get all games from DynamoDB
    let games = dynamodb.list_games().await?;

    // Build response
    let response = ListGamesResponse {
        total: games.len(),
        games,
    };

    let response_body = serde_json::to_string(&response)
        .map_err(|e| AdminError::InternalError(format!("Failed to serialize response: {}", e)))?;

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(response_body.into())
        .unwrap())
}

/// Handle GET /api/admin/games/{id} - Get specific game
pub async fn handle_get_game(
    request: Request,
    jwt_validator: &JwtValidator,
    dynamodb: &DynamoDbService,
    game_id: &str,
) -> AdminResult<Response<Body>> {
    info!("Handling get game request for: {}", game_id);

    // Verify admin authorization
    verify_admin(&request, jwt_validator, dynamodb).await?;

    // Get game from DynamoDB
    let game = dynamodb.get_game(game_id).await?;

    let response_body = serde_json::to_string(&game)
        .map_err(|e| AdminError::InternalError(format!("Failed to serialize response: {}", e)))?;

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(response_body.into())
        .unwrap())
}

/// Handle POST /api/admin/games - Create new game
pub async fn handle_create_game(
    request: Request,
    jwt_validator: &JwtValidator,
    dynamodb: &DynamoDbService,
) -> AdminResult<Response<Body>> {
    info!("Handling create game request");

    // Verify admin authorization
    verify_admin(&request, jwt_validator, dynamodb).await?;

    // Parse request body
    let body = request.body();
    let create_req: CreateGameRequest = serde_json::from_slice(body.as_ref())
        .map_err(|e| AdminError::InvalidRequest(format!("Invalid JSON: {}", e)))?;

    // Validate required fields
    if create_req.game_id.is_empty() {
        return Err(AdminError::InvalidRequest(
            "game_id is required".to_string(),
        ));
    }
    if create_req.title.is_empty() {
        return Err(AdminError::InvalidRequest("title is required".to_string()));
    }
    if create_req.author.is_empty() {
        return Err(AdminError::InvalidRequest("author is required".to_string()));
    }

    // Create game in DynamoDB
    let game = dynamodb.create_game(&create_req).await?;

    let response_body = serde_json::to_string(&game)
        .map_err(|e| AdminError::InternalError(format!("Failed to serialize response: {}", e)))?;

    Ok(Response::builder()
        .status(201)
        .header("Content-Type", "application/json")
        .body(response_body.into())
        .unwrap())
}

/// Handle PUT /api/admin/games/{id} - Update game
pub async fn handle_update_game(
    request: Request,
    jwt_validator: &JwtValidator,
    dynamodb: &DynamoDbService,
    game_id: &str,
) -> AdminResult<Response<Body>> {
    info!("Handling update game request for: {}", game_id);

    // Verify admin authorization
    verify_admin(&request, jwt_validator, dynamodb).await?;

    // Parse request body
    let body = request.body();
    let update_req: UpdateGameRequest = serde_json::from_slice(body.as_ref())
        .map_err(|e| AdminError::InvalidRequest(format!("Invalid JSON: {}", e)))?;

    // Validate required fields
    if update_req.title.is_empty() {
        return Err(AdminError::InvalidRequest("title is required".to_string()));
    }
    if update_req.author.is_empty() {
        return Err(AdminError::InvalidRequest("author is required".to_string()));
    }

    // Update game in DynamoDB
    let game = dynamodb.update_game(game_id, &update_req).await?;

    let response_body = serde_json::to_string(&game)
        .map_err(|e| AdminError::InternalError(format!("Failed to serialize response: {}", e)))?;

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(response_body.into())
        .unwrap())
}

/// Handle DELETE /api/admin/games/{id} - Delete (archive) game
pub async fn handle_delete_game(
    request: Request,
    jwt_validator: &JwtValidator,
    dynamodb: &DynamoDbService,
    game_id: &str,
) -> AdminResult<Response<Body>> {
    info!("Handling delete game request for: {}", game_id);

    // Verify admin authorization
    verify_admin(&request, jwt_validator, dynamodb).await?;

    // Archive game in DynamoDB
    dynamodb.delete_game(game_id).await?;

    let response = SuccessResponse {
        message: format!("Game {} archived successfully", game_id),
    };

    let response_body = serde_json::to_string(&response)
        .map_err(|e| AdminError::InternalError(format!("Failed to serialize response: {}", e)))?;

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(response_body.into())
        .unwrap())
}

/// Handle POST /api/admin/games/upload-url - Generate presigned upload URL
pub async fn handle_upload_url(
    request: Request,
    jwt_validator: &JwtValidator,
    dynamodb: &DynamoDbService,
    s3: &S3Service,
) -> AdminResult<Response<Body>> {
    info!("Handling upload URL request");

    // Verify admin authorization
    verify_admin(&request, jwt_validator, dynamodb).await?;

    // Parse request body
    let body = request.body();
    let upload_req: UploadUrlRequest = serde_json::from_slice(body.as_ref())
        .map_err(|e| AdminError::InvalidRequest(format!("Invalid JSON: {}", e)))?;

    // Validate filename
    if upload_req.filename.is_empty() {
        return Err(AdminError::InvalidRequest(
            "filename is required".to_string(),
        ));
    }

    // Validate file extension
    let valid_extensions = ["z3", "z4", "z5", "z8"];
    let has_valid_ext = valid_extensions.iter().any(|ext| {
        upload_req
            .filename
            .to_lowercase()
            .ends_with(&format!(".{}", ext))
    });

    if !has_valid_ext {
        return Err(AdminError::InvalidRequest(
            "File must have .z3, .z4, .z5, or .z8 extension".to_string(),
        ));
    }

    // Generate presigned URL
    let (upload_url, s3_key) = s3.generate_upload_url(&upload_req.filename).await?;

    let response = UploadUrlResponse {
        upload_url,
        s3_key,
        expires_in: 300, // 5 minutes
    };

    let response_body = serde_json::to_string(&response)
        .map_err(|e| AdminError::InternalError(format!("Failed to serialize response: {}", e)))?;

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(response_body.into())
        .unwrap())
}
