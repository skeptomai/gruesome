use aws_sdk_dynamodb::types::AttributeValue;
use aws_sdk_dynamodb::Client as DynamoDbClient;
use lambda_http::{Body, Request, Response};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

use crate::auth::require_admin;
use crate::error::ApiError;
use crate::validation::{
    validate_author, validate_category, validate_description, validate_game_id, validate_title,
    validate_year,
};

#[derive(Debug, Deserialize)]
pub struct CreateGameRequest {
    pub game_id: String,
    pub title: String,
    pub author: String,
    pub description: String,
    pub category: Option<String>,
    pub year_published: Option<i32>,
    pub s3_key: String,
    pub file_size: u32,
    pub version: u8,
    pub release: u16,
    pub serial: String,
    pub checksum: String,
}

#[derive(Debug, Serialize)]
pub struct GameResponse {
    pub game_id: String,
    pub title: String,
    pub author: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year_published: Option<i32>,
    pub s3_key: String,
    pub file_size: u32,
    pub version: u8,
    pub release: u16,
    pub serial: String,
    pub checksum: String,
    pub created_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_at: Option<i64>,
    pub archived: bool,
}

#[derive(Debug, Serialize)]
pub struct CreateGameResponse {
    pub success: bool,
    pub game_id: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ListGamesResponse {
    pub games: Vec<GameResponse>,
    pub total: usize,
}

/// Handle POST /api/admin/games
/// Create a new game metadata entry
pub async fn handle_create_game(event: Request) -> Result<Response<Body>, ApiError> {
    let config = aws_config::load_from_env().await;
    let dynamodb = DynamoDbClient::new(&config);
    let table_name = std::env::var("TABLE_NAME").unwrap_or_else(|_| "gruesome-platform".to_string());

    // Verify admin role
    let user_id = require_admin(&event, &dynamodb, &table_name).await?;
    info!("Admin user {} creating game", user_id);

    // Parse request body
    let body = event.body();
    let request: CreateGameRequest = serde_json::from_slice(body)?;

    // Validate all fields
    validate_game_id(&request.game_id)?;
    validate_title(&request.title)?;
    validate_author(&request.author)?;
    validate_description(&request.description)?;
    validate_category(request.category.as_deref())?;
    validate_year(request.year_published)?;

    // Check if game already exists
    let existing = dynamodb
        .get_item()
        .table_name(&table_name)
        .key("PK", AttributeValue::S(format!("GAME#{}", request.game_id)))
        .key("SK", AttributeValue::S("METADATA".to_string()))
        .send()
        .await
        .map_err(|e| ApiError::DynamoDbError(format!("Failed to check existing game: {:?}", e)))?;

    if existing.item().is_some() {
        return Err(ApiError::BadRequest(format!(
            "Game with ID '{}' already exists",
            request.game_id
        )));
    }

    // Create timestamp
    let now = chrono::Utc::now().timestamp();

    // Build DynamoDB item
    let mut item = HashMap::new();
    item.insert("PK".to_string(), AttributeValue::S(format!("GAME#{}", request.game_id)));
    item.insert("SK".to_string(), AttributeValue::S("METADATA".to_string()));
    item.insert("entity_type".to_string(), AttributeValue::S("GAME".to_string()));
    item.insert("game_id".to_string(), AttributeValue::S(request.game_id.clone()));
    item.insert("title".to_string(), AttributeValue::S(request.title));
    item.insert("author".to_string(), AttributeValue::S(request.author));
    item.insert("description".to_string(), AttributeValue::S(request.description));

    if let Some(category) = request.category {
        item.insert("category".to_string(), AttributeValue::S(category));
    }

    if let Some(year) = request.year_published {
        item.insert("year_published".to_string(), AttributeValue::N(year.to_string()));
    }

    item.insert("s3_key".to_string(), AttributeValue::S(request.s3_key));
    item.insert("file_size".to_string(), AttributeValue::N(request.file_size.to_string()));
    item.insert("version".to_string(), AttributeValue::N(request.version.to_string()));
    item.insert("release".to_string(), AttributeValue::N(request.release.to_string()));
    item.insert("serial".to_string(), AttributeValue::S(request.serial));
    item.insert("checksum".to_string(), AttributeValue::S(request.checksum));
    item.insert("created_at".to_string(), AttributeValue::N(now.to_string()));
    item.insert("created_by".to_string(), AttributeValue::S(user_id.clone()));
    item.insert("modified_at".to_string(), AttributeValue::N(now.to_string()));
    item.insert("modified_by".to_string(), AttributeValue::S(user_id.clone()));
    item.insert("archived".to_string(), AttributeValue::Bool(false));

    // Put item in DynamoDB
    dynamodb
        .put_item()
        .table_name(&table_name)
        .set_item(Some(item))
        .send()
        .await
        .map_err(|e| ApiError::DynamoDbError(format!("Failed to create game: {:?}", e)))?;

    info!("Successfully created game: {}", request.game_id);

    // Return success response
    let response = CreateGameResponse {
        success: true,
        game_id: request.game_id.clone(),
        message: format!("Game '{}' created successfully", request.game_id),
    };

    let body = serde_json::to_string(&response)?;

    Ok(Response::builder()
        .status(201)
        .header("Content-Type", "application/json")
        .body(Body::Text(body))
        .unwrap())
}

/// Handle GET /api/admin/games
/// List all games (including archived)
pub async fn handle_list_games(event: Request) -> Result<Response<Body>, ApiError> {
    let config = aws_config::load_from_env().await;
    let dynamodb = DynamoDbClient::new(&config);
    let table_name = std::env::var("TABLE_NAME").unwrap_or_else(|_| "gruesome-platform".to_string());

    // Verify admin role
    let _user_id = require_admin(&event, &dynamodb, &table_name).await?;

    // Query all games using Scan with filter
    let result = dynamodb
        .scan()
        .table_name(&table_name)
        .filter_expression("begins_with(PK, :pk)")
        .expression_attribute_values(":pk", AttributeValue::S("GAME#".to_string()))
        .send()
        .await
        .map_err(|e| ApiError::DynamoDbError(format!("Failed to list games: {:?}", e)))?;

    let items = result.items().to_vec();
    let mut games = Vec::new();

    for item in items {
        if let Some(game) = parse_game_item(&item) {
            games.push(game);
        }
    }

    // Sort by title
    games.sort_by(|a, b| a.title.cmp(&b.title));

    let response = ListGamesResponse {
        total: games.len(),
        games,
    };

    let body = serde_json::to_string(&response)?;

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(Body::Text(body))
        .unwrap())
}

/// Handle GET /api/admin/games/{id}
/// Get a specific game's metadata
pub async fn handle_get_game(event: Request, game_id: &str) -> Result<Response<Body>, ApiError> {
    let config = aws_config::load_from_env().await;
    let dynamodb = DynamoDbClient::new(&config);
    let table_name = std::env::var("TABLE_NAME").unwrap_or_else(|_| "gruesome-platform".to_string());

    // Verify admin role
    let _user_id = require_admin(&event, &dynamodb, &table_name).await?;

    // Validate game_id
    validate_game_id(game_id)?;

    // Get game from DynamoDB
    let result = dynamodb
        .get_item()
        .table_name(&table_name)
        .key("PK", AttributeValue::S(format!("GAME#{}", game_id)))
        .key("SK", AttributeValue::S("METADATA".to_string()))
        .send()
        .await
        .map_err(|e| ApiError::DynamoDbError(format!("Failed to get game: {:?}", e)))?;

    let item = result.item().ok_or(ApiError::NotFound)?;

    let game = parse_game_item(item).ok_or(ApiError::InternalError(
        "Failed to parse game item".to_string(),
    ))?;

    let body = serde_json::to_string(&game)?;

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(Body::Text(body))
        .unwrap())
}

/// Handle PUT /api/admin/games/{id}
/// Update game metadata
pub async fn handle_update_game(event: Request, game_id: &str) -> Result<Response<Body>, ApiError> {
    let config = aws_config::load_from_env().await;
    let dynamodb = DynamoDbClient::new(&config);
    let table_name = std::env::var("TABLE_NAME").unwrap_or_else(|_| "gruesome-platform".to_string());

    // Verify admin role
    let user_id = require_admin(&event, &dynamodb, &table_name).await?;

    // Validate game_id
    validate_game_id(game_id)?;

    // Parse request body
    let body = event.body();
    let request: CreateGameRequest = serde_json::from_slice(body)?;

    // Validate fields
    validate_title(&request.title)?;
    validate_author(&request.author)?;
    validate_description(&request.description)?;
    validate_category(request.category.as_deref())?;
    validate_year(request.year_published)?;

    // Check if game exists
    let existing = dynamodb
        .get_item()
        .table_name(&table_name)
        .key("PK", AttributeValue::S(format!("GAME#{}", game_id)))
        .key("SK", AttributeValue::S("METADATA".to_string()))
        .send()
        .await
        .map_err(|e| ApiError::DynamoDbError(format!("Failed to get game: {:?}", e)))?;

    if existing.item().is_none() {
        return Err(ApiError::NotFound);
    }

    // Update timestamp
    let now = chrono::Utc::now().timestamp();

    // Build update expression
    let mut update_parts = vec![
        "title = :title",
        "author = :author",
        "description = :description",
        "modified_at = :modified_at",
        "modified_by = :modified_by",
    ];

    let mut attr_values = HashMap::new();
    attr_values.insert(":title".to_string(), AttributeValue::S(request.title));
    attr_values.insert(":author".to_string(), AttributeValue::S(request.author));
    attr_values.insert(":description".to_string(), AttributeValue::S(request.description));
    attr_values.insert(":modified_at".to_string(), AttributeValue::N(now.to_string()));
    attr_values.insert(":modified_by".to_string(), AttributeValue::S(user_id.clone()));

    if let Some(category) = request.category {
        update_parts.push("category = :category");
        attr_values.insert(":category".to_string(), AttributeValue::S(category));
    }

    if let Some(year) = request.year_published {
        update_parts.push("year_published = :year");
        attr_values.insert(":year".to_string(), AttributeValue::N(year.to_string()));
    }

    let update_expression = format!("SET {}", update_parts.join(", "));

    // Update item
    dynamodb
        .update_item()
        .table_name(&table_name)
        .key("PK", AttributeValue::S(format!("GAME#{}", game_id)))
        .key("SK", AttributeValue::S("METADATA".to_string()))
        .update_expression(update_expression)
        .set_expression_attribute_values(Some(attr_values))
        .send()
        .await
        .map_err(|e| ApiError::DynamoDbError(format!("Failed to update game: {:?}", e)))?;

    info!("Successfully updated game: {}", game_id);

    let response = CreateGameResponse {
        success: true,
        game_id: game_id.to_string(),
        message: format!("Game '{}' updated successfully", game_id),
    };

    let body = serde_json::to_string(&response)?;

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(Body::Text(body))
        .unwrap())
}

/// Handle DELETE /api/admin/games/{id}
/// Soft delete a game (set archived = true)
pub async fn handle_delete_game(event: Request, game_id: &str) -> Result<Response<Body>, ApiError> {
    let config = aws_config::load_from_env().await;
    let dynamodb = DynamoDbClient::new(&config);
    let table_name = std::env::var("TABLE_NAME").unwrap_or_else(|_| "gruesome-platform".to_string());

    // Verify admin role
    let user_id = require_admin(&event, &dynamodb, &table_name).await?;

    // Validate game_id
    validate_game_id(game_id)?;

    // Check if game exists
    let existing = dynamodb
        .get_item()
        .table_name(&table_name)
        .key("PK", AttributeValue::S(format!("GAME#{}", game_id)))
        .key("SK", AttributeValue::S("METADATA".to_string()))
        .send()
        .await
        .map_err(|e| ApiError::DynamoDbError(format!("Failed to get game: {:?}", e)))?;

    if existing.item().is_none() {
        return Err(ApiError::NotFound);
    }

    // Soft delete: set archived = true
    let now = chrono::Utc::now().timestamp();

    dynamodb
        .update_item()
        .table_name(&table_name)
        .key("PK", AttributeValue::S(format!("GAME#{}", game_id)))
        .key("SK", AttributeValue::S("METADATA".to_string()))
        .update_expression("SET archived = :archived, modified_at = :modified_at, modified_by = :modified_by")
        .expression_attribute_values(":archived", AttributeValue::Bool(true))
        .expression_attribute_values(":modified_at", AttributeValue::N(now.to_string()))
        .expression_attribute_values(":modified_by", AttributeValue::S(user_id))
        .send()
        .await
        .map_err(|e| ApiError::DynamoDbError(format!("Failed to delete game: {:?}", e)))?;

    info!("Successfully soft-deleted game: {}", game_id);

    let response = CreateGameResponse {
        success: true,
        game_id: game_id.to_string(),
        message: format!("Game '{}' deleted successfully", game_id),
    };

    let body = serde_json::to_string(&response)?;

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(Body::Text(body))
        .unwrap())
}

/// Helper function to parse DynamoDB item into GameResponse
fn parse_game_item(item: &HashMap<String, AttributeValue>) -> Option<GameResponse> {
    Some(GameResponse {
        game_id: item.get("game_id")?.as_s().ok()?.clone(),
        title: item.get("title")?.as_s().ok()?.clone(),
        author: item.get("author")?.as_s().ok()?.clone(),
        description: item.get("description")?.as_s().ok()?.clone(),
        category: item.get("category").and_then(|v| v.as_s().ok().cloned()),
        year_published: item.get("year_published").and_then(|v| v.as_n().ok()?.parse().ok()),
        s3_key: item.get("s3_key")?.as_s().ok()?.clone(),
        file_size: item.get("file_size")?.as_n().ok()?.parse().ok()?,
        version: item.get("version")?.as_n().ok()?.parse().ok()?,
        release: item.get("release")?.as_n().ok()?.parse().ok()?,
        serial: item.get("serial")?.as_s().ok()?.clone(),
        checksum: item.get("checksum")?.as_s().ok()?.clone(),
        created_at: item.get("created_at")?.as_n().ok()?.parse().ok()?,
        modified_at: item.get("modified_at").and_then(|v| v.as_n().ok()?.parse().ok()),
        archived: item.get("archived").and_then(|v| v.as_bool().ok().copied()).unwrap_or(false),
    })
}
