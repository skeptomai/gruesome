use lambda_http::{Request, Response, Body, RequestExt};

use crate::error::GameError;
use crate::game_service::GameService;
use crate::save_service::SaveService;
use crate::models::*;
use crate::jwt_auth::JwtValidator;

/// Extract JWT from Authorization header and validate
fn extract_user_id(request: &Request, jwt_validator: &JwtValidator) -> Result<String, GameError> {
    let auth_header = request
        .headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or(GameError::Unauthorized)?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(GameError::Unauthorized)?;

    jwt_validator.validate_token(token)
        .map_err(|_| GameError::Unauthorized)
}

/// Parse JSON request body
fn parse_body<T: serde::de::DeserializeOwned>(request: &Request) -> Result<T, GameError> {
    let body = request.body();
    serde_json::from_slice(body.as_ref())
        .map_err(|e| GameError::InvalidRequest(format!("Invalid JSON: {}", e)))
}

/// Extract path parameter
fn get_path_param(request: &Request, name: &str) -> Result<String, GameError> {
    request
        .path_parameters()
        .first(name)
        .ok_or_else(|| GameError::InvalidRequest(format!("Missing path parameter: {}", name)))
        .map(|s| s.to_string())
}

// ========== Game Handlers ==========

/// GET /api/games - List all available games
pub async fn handle_list_games(
    game_service: &GameService,
) -> Result<Response<Body>, GameError> {
    let games = game_service.list_games().await?;

    let response = ListGamesResponse { games };
    let json = serde_json::to_string(&response)
        .map_err(|e| GameError::InternalError(format!("JSON serialization error: {}", e)))?;

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(json.into())
        .unwrap())
}

/// GET /api/games/{game_id} - Get specific game metadata
pub async fn handle_get_game(
    request: Request,
    game_service: &GameService,
) -> Result<Response<Body>, GameError> {
    let game_id = get_path_param(&request, "game_id")?;
    let game = game_service.get_game(&game_id).await?;

    let response = GetGameResponse { game };
    let json = serde_json::to_string(&response)
        .map_err(|e| GameError::InternalError(format!("JSON serialization error: {}", e)))?;

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(json.into())
        .unwrap())
}

/// GET /api/games/{game_id}/file - Get presigned URL to download game file
pub async fn handle_get_game_file(
    request: Request,
    game_service: &GameService,
) -> Result<Response<Body>, GameError> {
    let game_id = get_path_param(&request, "game_id")?;
    let download_url = game_service.get_game_file_url(&game_id).await?;

    let response = GetGameFileResponse {
        download_url,
        expires_in: 300,
    };
    let json = serde_json::to_string(&response)
        .map_err(|e| GameError::InternalError(format!("JSON serialization error: {}", e)))?;

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(json.into())
        .unwrap())
}

// ========== Save Handlers ==========

/// GET /api/saves - List all saves for authenticated user
pub async fn handle_list_saves(
    request: Request,
    save_service: &SaveService,
    jwt_validator: &JwtValidator,
) -> Result<Response<Body>, GameError> {
    let user_id = extract_user_id(&request, jwt_validator)?;
    let saves = save_service.list_user_saves(&user_id).await?;

    let response = ListSavesResponse { saves };
    let json = serde_json::to_string(&response)
        .map_err(|e| GameError::InternalError(format!("JSON serialization error: {}", e)))?;

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(json.into())
        .unwrap())
}

/// GET /api/saves/{game_id} - List saves for specific game
pub async fn handle_list_game_saves(
    request: Request,
    save_service: &SaveService,
    jwt_validator: &JwtValidator,
) -> Result<Response<Body>, GameError> {
    let user_id = extract_user_id(&request, jwt_validator)?;
    let game_id = get_path_param(&request, "game_id")?;

    let saves = save_service.list_game_saves(&user_id, &game_id).await?;

    let response = ListSavesResponse { saves };
    let json = serde_json::to_string(&response)
        .map_err(|e| GameError::InternalError(format!("JSON serialization error: {}", e)))?;

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(json.into())
        .unwrap())
}

/// GET /api/saves/{game_id}/{save_name} - Get presigned URL to download save
pub async fn handle_get_save(
    request: Request,
    save_service: &SaveService,
    jwt_validator: &JwtValidator,
) -> Result<Response<Body>, GameError> {
    let user_id = extract_user_id(&request, jwt_validator)?;
    let game_id = get_path_param(&request, "game_id")?;
    let save_name = get_path_param(&request, "save_name")?;

    let download_url = save_service.get_save_download_url(&user_id, &game_id, &save_name).await?;

    let response = GetSaveDownloadResponse {
        download_url,
        expires_in: 300,
    };
    let json = serde_json::to_string(&response)
        .map_err(|e| GameError::InternalError(format!("JSON serialization error: {}", e)))?;

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(json.into())
        .unwrap())
}

/// POST /api/saves/{game_id}/{save_name} - Get presigned URL to upload save
pub async fn handle_create_save(
    request: Request,
    save_service: &SaveService,
    jwt_validator: &JwtValidator,
) -> Result<Response<Body>, GameError> {
    let user_id = extract_user_id(&request, jwt_validator)?;
    let game_id = get_path_param(&request, "game_id")?;
    let save_name = get_path_param(&request, "save_name")?;

    let req_body: CreateSaveRequest = parse_body(&request).unwrap_or(CreateSaveRequest { file_size: None });

    let upload_url = save_service
        .get_save_upload_url(&user_id, &game_id, &save_name, req_body.file_size)
        .await?;

    let response = CreateSaveResponse {
        upload_url,
        expires_in: 300,
    };
    let json = serde_json::to_string(&response)
        .map_err(|e| GameError::InternalError(format!("JSON serialization error: {}", e)))?;

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(json.into())
        .unwrap())
}

/// DELETE /api/saves/{game_id}/{save_name} - Delete save file
pub async fn handle_delete_save(
    request: Request,
    save_service: &SaveService,
    jwt_validator: &JwtValidator,
) -> Result<Response<Body>, GameError> {
    let user_id = extract_user_id(&request, jwt_validator)?;
    let game_id = get_path_param(&request, "game_id")?;
    let save_name = get_path_param(&request, "save_name")?;

    save_service.delete_save(&user_id, &game_id, &save_name).await?;

    let response = DeleteSaveResponse { deleted: true };
    let json = serde_json::to_string(&response)
        .map_err(|e| GameError::InternalError(format!("JSON serialization error: {}", e)))?;

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(json.into())
        .unwrap())
}
