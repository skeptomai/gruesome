use aws_sdk_dynamodb::{types::AttributeValue, Client};
use std::collections::HashMap;
use tracing::{error, info};

use crate::error::{AdminError, AdminResult};
use crate::models::{CreateGameRequest, GameMetadata, UpdateGameRequest};

pub struct DynamoDbService {
    client: Client,
    table_name: String,
}

impl DynamoDbService {
    pub fn new(client: Client, table_name: String) -> Self {
        Self { client, table_name }
    }

    /// Check if user has admin role
    pub async fn check_admin_role(&self, user_id: &str) -> AdminResult<bool> {
        info!("Checking admin role for user: {}", user_id);

        let pk = format!("USER#{}", user_id);
        let sk = "PROFILE";

        let result = self
            .client
            .get_item()
            .table_name(&self.table_name)
            .key("PK", AttributeValue::S(pk))
            .key("SK", AttributeValue::S(sk.to_string()))
            .send()
            .await
            .map_err(|e| {
                error!("DynamoDB GetItem error: {:?}", e);
                AdminError::DynamoDbError(e.to_string())
            })?;

        let item = result
            .item()
            .ok_or_else(|| AdminError::Unauthorized("User not found".to_string()))?;

        let is_admin = item
            .get("role")
            .and_then(|v| v.as_s().ok())
            .map(|role| role == "admin")
            .unwrap_or(false);

        Ok(is_admin)
    }

    /// List all games (including archived)
    pub async fn list_games(&self) -> AdminResult<Vec<GameMetadata>> {
        info!("Listing all games");

        let result = self
            .client
            .scan()
            .table_name(&self.table_name)
            .filter_expression("begins_with(PK, :pk) AND begins_with(SK, :sk)")
            .expression_attribute_values(":pk", AttributeValue::S("GAME#".to_string()))
            .expression_attribute_values(":sk", AttributeValue::S("METADATA".to_string()))
            .send()
            .await
            .map_err(|e| {
                error!("DynamoDB Scan error: {:?}", e);
                AdminError::DynamoDbError(e.to_string())
            })?;

        let items = result.items();
        let mut games = Vec::new();

        for item in items {
            if let Ok(game) = self.item_to_game_metadata(item) {
                games.push(game);
            }
        }

        // Sort by display_order (nulls/missing go to end), then by title
        games.sort_by(|a, b| match (a.display_order, b.display_order) {
            (Some(a_order), Some(b_order)) => a_order.cmp(&b_order),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.title.cmp(&b.title),
        });

        info!("Found {} games", games.len());
        Ok(games)
    }

    /// Get specific game by ID
    pub async fn get_game(&self, game_id: &str) -> AdminResult<GameMetadata> {
        info!("Getting game: {}", game_id);

        let pk = format!("GAME#{}", game_id);
        let sk = "METADATA";

        let result = self
            .client
            .get_item()
            .table_name(&self.table_name)
            .key("PK", AttributeValue::S(pk))
            .key("SK", AttributeValue::S(sk.to_string()))
            .send()
            .await
            .map_err(|e| {
                error!("DynamoDB GetItem error: {:?}", e);
                AdminError::DynamoDbError(e.to_string())
            })?;

        let item = result
            .item()
            .ok_or_else(|| AdminError::NotFound(format!("Game {} not found", game_id)))?;

        self.item_to_game_metadata(item)
    }

    /// Create new game metadata
    pub async fn create_game(&self, request: &CreateGameRequest) -> AdminResult<GameMetadata> {
        info!("Creating game: {}", request.game_id);

        let now = chrono::Utc::now().to_rfc3339();

        let mut item = HashMap::new();
        item.insert(
            "PK".to_string(),
            AttributeValue::S(format!("GAME#{}", request.game_id)),
        );
        item.insert("SK".to_string(), AttributeValue::S("METADATA".to_string()));
        item.insert(
            "game_id".to_string(),
            AttributeValue::S(request.game_id.clone()),
        );
        item.insert(
            "title".to_string(),
            AttributeValue::S(request.title.clone()),
        );
        item.insert(
            "author".to_string(),
            AttributeValue::S(request.author.clone()),
        );
        item.insert(
            "description".to_string(),
            AttributeValue::S(request.description.clone()),
        );

        if let Some(ref category) = request.category {
            item.insert("category".to_string(), AttributeValue::S(category.clone()));
        }

        if let Some(year) = request.year {
            item.insert("year".to_string(), AttributeValue::N(year.to_string()));
        }

        item.insert(
            "version".to_string(),
            AttributeValue::N(request.version.to_string()),
        );
        item.insert(
            "release".to_string(),
            AttributeValue::N(request.release.to_string()),
        );
        item.insert(
            "serial".to_string(),
            AttributeValue::S(request.serial.clone()),
        );
        item.insert(
            "checksum".to_string(),
            AttributeValue::S(request.checksum.clone()),
        );
        item.insert(
            "file_size".to_string(),
            AttributeValue::N(request.file_size.to_string()),
        );
        item.insert(
            "s3_key".to_string(),
            AttributeValue::S(request.s3_key.clone()),
        );
        item.insert("created_at".to_string(), AttributeValue::S(now.clone()));
        item.insert("updated_at".to_string(), AttributeValue::S(now.clone()));
        item.insert("archived".to_string(), AttributeValue::Bool(false));
        item.insert(
            "entity_type".to_string(),
            AttributeValue::S("GAME".to_string()),
        );

        if let Some(display_order) = request.display_order {
            item.insert(
                "display_order".to_string(),
                AttributeValue::N(display_order.to_string()),
            );
        }

        self.client
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(item))
            .send()
            .await
            .map_err(|e| {
                error!("DynamoDB PutItem error: {:?}", e);
                AdminError::DynamoDbError(e.to_string())
            })?;

        info!("Game created successfully");
        self.get_game(&request.game_id).await
    }

    /// Update game metadata
    pub async fn update_game(
        &self,
        game_id: &str,
        request: &UpdateGameRequest,
    ) -> AdminResult<GameMetadata> {
        info!("Updating game: {}", game_id);

        let pk = format!("GAME#{}", game_id);
        let sk = "METADATA";
        let now = chrono::Utc::now().to_rfc3339();

        let mut update_expr = vec![
            "title = :title".to_string(),
            "author = :author".to_string(),
            "description = :description".to_string(),
            "updated_at = :updated_at".to_string(),
        ];

        let mut expr_values = HashMap::new();
        expr_values.insert(
            ":title".to_string(),
            AttributeValue::S(request.title.clone()),
        );
        expr_values.insert(
            ":author".to_string(),
            AttributeValue::S(request.author.clone()),
        );
        expr_values.insert(
            ":description".to_string(),
            AttributeValue::S(request.description.clone()),
        );
        expr_values.insert(":updated_at".to_string(), AttributeValue::S(now));

        if let Some(ref category) = request.category {
            update_expr.push("category = :category".to_string());
            expr_values.insert(":category".to_string(), AttributeValue::S(category.clone()));
        }

        if let Some(year) = request.year {
            update_expr.push("year = :year".to_string());
            expr_values.insert(":year".to_string(), AttributeValue::N(year.to_string()));
        }

        if let Some(display_order) = request.display_order {
            update_expr.push("display_order = :display_order".to_string());
            expr_values.insert(
                ":display_order".to_string(),
                AttributeValue::N(display_order.to_string()),
            );
        }

        let update_expression = format!("SET {}", update_expr.join(", "));

        self.client
            .update_item()
            .table_name(&self.table_name)
            .key("PK", AttributeValue::S(pk))
            .key("SK", AttributeValue::S(sk.to_string()))
            .update_expression(update_expression)
            .set_expression_attribute_values(Some(expr_values))
            .send()
            .await
            .map_err(|e| {
                error!("DynamoDB UpdateItem error: {:?}", e);
                AdminError::DynamoDbError(e.to_string())
            })?;

        info!("Game updated successfully");
        self.get_game(game_id).await
    }

    /// Delete (archive) game
    pub async fn delete_game(&self, game_id: &str) -> AdminResult<()> {
        info!("Archiving game: {}", game_id);

        let pk = format!("GAME#{}", game_id);
        let sk = "METADATA";
        let now = chrono::Utc::now().to_rfc3339();

        self.client
            .update_item()
            .table_name(&self.table_name)
            .key("PK", AttributeValue::S(pk))
            .key("SK", AttributeValue::S(sk.to_string()))
            .update_expression("SET archived = :archived, updated_at = :updated_at")
            .expression_attribute_values(":archived", AttributeValue::Bool(true))
            .expression_attribute_values(":updated_at", AttributeValue::S(now))
            .send()
            .await
            .map_err(|e| {
                error!("DynamoDB UpdateItem error: {:?}", e);
                AdminError::DynamoDbError(e.to_string())
            })?;

        info!("Game archived successfully");
        Ok(())
    }

    /// Convert DynamoDB item to GameMetadata
    fn item_to_game_metadata(
        &self,
        item: &HashMap<String, AttributeValue>,
    ) -> AdminResult<GameMetadata> {
        // Handle created_at - can be either Number (Unix timestamp) or String (RFC3339)
        let created_at = if let Some(timestamp) = get_optional_number_attr(item, "created_at") {
            // Convert Unix timestamp to RFC3339
            chrono::DateTime::from_timestamp(timestamp, 0)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_else(|| chrono::Utc::now().to_rfc3339())
        } else {
            get_optional_string_attr(item, "created_at")
                .unwrap_or_else(|| chrono::Utc::now().to_rfc3339())
        };

        // Handle updated_at with same logic
        let updated_at = if let Some(timestamp) = get_optional_number_attr(item, "updated_at") {
            chrono::DateTime::from_timestamp(timestamp, 0)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_else(|| created_at.clone())
        } else {
            get_optional_string_attr(item, "updated_at").unwrap_or_else(|| created_at.clone())
        };

        Ok(GameMetadata {
            game_id: get_string_attr(item, "game_id")?,
            title: get_string_attr(item, "title")?,
            author: get_string_attr(item, "author")?,
            description: get_string_attr(item, "description")?,
            category: get_optional_string_attr(item, "category"),
            year: get_optional_number_attr(item, "year").map(|n| n as i32),
            version: get_number_attr(item, "version")? as i32,
            release: get_optional_number_attr(item, "release").unwrap_or(0) as i32,
            serial: get_optional_string_attr(item, "serial")
                .unwrap_or_else(|| "000000".to_string()),
            checksum: get_optional_string_attr(item, "checksum")
                .unwrap_or_else(|| "0000".to_string()),
            file_size: get_number_attr(item, "file_size")?,
            s3_key: get_string_attr(item, "s3_key")?,
            created_at,
            updated_at,
            archived: get_bool_attr(item, "archived").unwrap_or(false),
            display_order: get_optional_number_attr(item, "display_order").map(|n| n as i32),
        })
    }
}

/// Helper functions to extract attributes from DynamoDB items
fn get_string_attr(item: &HashMap<String, AttributeValue>, key: &str) -> AdminResult<String> {
    item.get(key)
        .and_then(|v| v.as_s().ok())
        .map(|s| s.to_string())
        .ok_or_else(|| AdminError::InternalError(format!("Missing or invalid attribute: {}", key)))
}

fn get_optional_string_attr(item: &HashMap<String, AttributeValue>, key: &str) -> Option<String> {
    item.get(key)
        .and_then(|v| v.as_s().ok())
        .map(|s| s.to_string())
}

fn get_number_attr(item: &HashMap<String, AttributeValue>, key: &str) -> AdminResult<i64> {
    item.get(key)
        .and_then(|v| v.as_n().ok())
        .and_then(|s| s.parse::<i64>().ok())
        .ok_or_else(|| {
            AdminError::InternalError(format!("Missing or invalid number attribute: {}", key))
        })
}

fn get_optional_number_attr(item: &HashMap<String, AttributeValue>, key: &str) -> Option<i64> {
    item.get(key)
        .and_then(|v| v.as_n().ok())
        .and_then(|s| s.parse::<i64>().ok())
}

fn get_bool_attr(item: &HashMap<String, AttributeValue>, key: &str) -> Option<bool> {
    item.get(key).and_then(|v| v.as_bool().ok()).copied()
}
