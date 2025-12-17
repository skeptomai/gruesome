use aws_sdk_dynamodb::{Client as DynamoClient, types::AttributeValue};
use aws_sdk_s3::{Client as S3Client, presigning::PresigningConfig};
use std::time::Duration;

use crate::error::GameError;
use crate::models::GameMetadata;

pub struct GameService {
    dynamodb_client: DynamoClient,
    s3_client: S3Client,
    table_name: String,
    games_bucket: String,
}

impl GameService {
    pub fn new(
        dynamodb_client: DynamoClient,
        s3_client: S3Client,
        table_name: String,
        games_bucket: String,
    ) -> Self {
        GameService {
            dynamodb_client,
            s3_client,
            table_name,
            games_bucket,
        }
    }

    /// List all available games
    pub async fn list_games(&self) -> Result<Vec<GameMetadata>, GameError> {
        let result = self.dynamodb_client
            .query()
            .table_name(&self.table_name)
            .index_name("entity-type-index")
            .key_condition_expression("entity_type = :entity_type")
            .expression_attribute_values(":entity_type", AttributeValue::S("GAME".to_string()))
            .send()
            .await
            .map_err(|e| GameError::AwsError(format!("DynamoDB query error: {}", e)))?;

        let mut games = Vec::new();
        if let Some(items) = result.items {
            for item in items {
                games.push(Self::parse_game_metadata(item)?);
            }
        }

        Ok(games)
    }

    /// Get metadata for a specific game
    pub async fn get_game(&self, game_id: &str) -> Result<GameMetadata, GameError> {
        let result = self.dynamodb_client
            .get_item()
            .table_name(&self.table_name)
            .key("PK", AttributeValue::S(format!("GAME#{}", game_id)))
            .key("SK", AttributeValue::S("METADATA".to_string()))
            .send()
            .await
            .map_err(|e| GameError::AwsError(format!("DynamoDB error: {}", e)))?;

        let item = result.item.ok_or(GameError::GameNotFound(game_id.to_string()))?;
        Self::parse_game_metadata(item)
    }

    /// Get presigned URL to download game file from S3
    pub async fn get_game_file_url(&self, game_id: &str) -> Result<String, GameError> {
        // First verify game exists in DynamoDB
        let game = self.get_game(game_id).await?;

        // Generate presigned URL for S3 download (valid for 5 minutes)
        let presigning_config = PresigningConfig::expires_in(Duration::from_secs(300))
            .map_err(|e| GameError::InternalError(format!("Presigning config error: {}", e)))?;

        let presigned_request = self.s3_client
            .get_object()
            .bucket(&self.games_bucket)
            .key(&game.s3_key)
            .presigned(presigning_config)
            .await
            .map_err(|e| GameError::AwsError(format!("S3 presigning error: {}", e)))?;

        Ok(presigned_request.uri().to_string())
    }

    /// Parse DynamoDB item into GameMetadata
    fn parse_game_metadata(
        item: std::collections::HashMap<String, AttributeValue>,
    ) -> Result<GameMetadata, GameError> {
        let get_str = |key: &str| -> Result<String, GameError> {
            item.get(key)
                .and_then(|v| v.as_s().ok())
                .cloned()
                .ok_or_else(|| GameError::InternalError(format!("Missing field: {}", key)))
        };

        let get_num = |key: &str| -> Result<i64, GameError> {
            item.get(key)
                .and_then(|v| v.as_n().ok())
                .and_then(|s| s.parse::<i64>().ok())
                .ok_or_else(|| GameError::InternalError(format!("Missing or invalid field: {}", key)))
        };

        Ok(GameMetadata {
            game_id: get_str("game_id")?,
            title: get_str("title")?,
            author: get_str("author").unwrap_or_else(|_| "Unknown".to_string()),
            description: get_str("description").unwrap_or_else(|_| "".to_string()),
            version: get_num("version")? as u8,
            file_size: get_num("file_size")? as u64,
            s3_key: get_str("s3_key")?,
            created_at: get_num("created_at")?,
        })
    }
}
