use aws_sdk_dynamodb::{Client as DynamoClient, types::AttributeValue};
use aws_sdk_s3::{Client as S3Client, presigning::PresigningConfig};
use chrono::Utc;
use std::time::Duration;

use crate::error::GameError;
use crate::models::SaveMetadata;

pub struct SaveService {
    dynamodb_client: DynamoClient,
    s3_client: S3Client,
    table_name: String,
    saves_bucket: String,
}

impl SaveService {
    pub fn new(
        dynamodb_client: DynamoClient,
        s3_client: S3Client,
        table_name: String,
        saves_bucket: String,
    ) -> Self {
        SaveService {
            dynamodb_client,
            s3_client,
            table_name,
            saves_bucket,
        }
    }

    /// List all saves for a user
    pub async fn list_user_saves(&self, user_id: &str) -> Result<Vec<SaveMetadata>, GameError> {
        let result = self.dynamodb_client
            .query()
            .table_name(&self.table_name)
            .key_condition_expression("PK = :pk AND begins_with(SK, :sk_prefix)")
            .expression_attribute_values(":pk", AttributeValue::S(format!("USER#{}", user_id)))
            .expression_attribute_values(":sk_prefix", AttributeValue::S("SAVE#".to_string()))
            .send()
            .await
            .map_err(|e| GameError::AwsError(format!("DynamoDB query error: {}", e)))?;

        let mut saves = Vec::new();
        if let Some(items) = result.items {
            for item in items {
                saves.push(Self::parse_save_metadata(item)?);
            }
        }

        Ok(saves)
    }

    /// List saves for a specific game
    pub async fn list_game_saves(&self, user_id: &str, game_id: &str) -> Result<Vec<SaveMetadata>, GameError> {
        let result = self.dynamodb_client
            .query()
            .table_name(&self.table_name)
            .key_condition_expression("PK = :pk AND begins_with(SK, :sk_prefix)")
            .expression_attribute_values(":pk", AttributeValue::S(format!("USER#{}", user_id)))
            .expression_attribute_values(":sk_prefix", AttributeValue::S(format!("SAVE#{}#", game_id)))
            .send()
            .await
            .map_err(|e| GameError::AwsError(format!("DynamoDB query error: {}", e)))?;

        let mut saves = Vec::new();
        if let Some(items) = result.items {
            for item in items {
                let save = Self::parse_save_metadata(item)?;

                // Validate S3 file exists to prevent orphaned saves
                let s3_exists = self.s3_client
                    .head_object()
                    .bucket(&self.saves_bucket)
                    .key(&save.s3_key)
                    .send()
                    .await
                    .is_ok();

                if s3_exists {
                    saves.push(save);
                } else {
                    eprintln!("WARNING: Orphaned save detected - metadata exists but S3 file missing: user_id={}, game_id={}, save_name={}, s3_key={}",
                        user_id, game_id, save.save_name, save.s3_key);
                }
            }
        }

        Ok(saves)
    }

    /// Get presigned URL to download a save file
    pub async fn get_save_download_url(
        &self,
        user_id: &str,
        game_id: &str,
        save_name: &str,
    ) -> Result<String, GameError> {
        // Verify save exists
        let save = self.get_save_metadata(user_id, game_id, save_name).await?;

        // Validate S3 file exists before generating presigned URL
        let s3_exists = self.s3_client
            .head_object()
            .bucket(&self.saves_bucket)
            .key(&save.s3_key)
            .send()
            .await
            .is_ok();

        if !s3_exists {
            eprintln!("ERROR: Orphaned save - metadata exists but S3 file missing: user_id={}, game_id={}, save_name={}, s3_key={}",
                user_id, game_id, save_name, save.s3_key);
            return Err(GameError::InternalError(format!(
                "Save file not found in storage. The save may have been corrupted or deleted."
            )));
        }

        // Generate presigned URL (valid for 5 minutes)
        let presigning_config = PresigningConfig::expires_in(Duration::from_secs(300))
            .map_err(|e| GameError::InternalError(format!("Presigning config error: {}", e)))?;

        let presigned_request = self.s3_client
            .get_object()
            .bucket(&self.saves_bucket)
            .key(&save.s3_key)
            .presigned(presigning_config)
            .await
            .map_err(|e| GameError::AwsError(format!("S3 presigning error: {}", e)))?;

        Ok(presigned_request.uri().to_string())
    }

    /// Get presigned URL to upload a save file
    pub async fn get_save_upload_url(
        &self,
        user_id: &str,
        game_id: &str,
        save_name: &str,
        file_size: Option<u64>,
    ) -> Result<String, GameError> {
        let now = Utc::now().timestamp();
        let s3_key = format!("{}/{}/{}.sav", user_id, game_id, save_name);

        // Check if save already exists
        let existing = self.get_save_metadata(user_id, game_id, save_name).await.ok();

        // Create or update metadata in DynamoDB
        self.dynamodb_client
            .put_item()
            .table_name(&self.table_name)
            .item("PK", AttributeValue::S(format!("USER#{}", user_id)))
            .item("SK", AttributeValue::S(format!("SAVE#{}#{}", game_id, save_name)))
            .item("entity_type", AttributeValue::S("SAVE".to_string()))
            .item("user_id", AttributeValue::S(user_id.to_string()))
            .item("game_id", AttributeValue::S(game_id.to_string()))
            .item("save_name", AttributeValue::S(save_name.to_string()))
            .item("s3_key", AttributeValue::S(s3_key.clone()))
            .item("file_size", AttributeValue::N(file_size.unwrap_or(0).to_string()))
            .item("created_at", AttributeValue::N(existing.map(|s| s.created_at).unwrap_or(now).to_string()))
            .item("last_updated", AttributeValue::N(now.to_string()))
            .send()
            .await
            .map_err(|e| GameError::AwsError(format!("DynamoDB error: {}", e)))?;

        // Generate presigned URL for upload (valid for 5 minutes)
        let presigning_config = PresigningConfig::expires_in(Duration::from_secs(300))
            .map_err(|e| GameError::InternalError(format!("Presigning config error: {}", e)))?;

        let presigned_request = self.s3_client
            .put_object()
            .bucket(&self.saves_bucket)
            .key(&s3_key)
            .presigned(presigning_config)
            .await
            .map_err(|e| GameError::AwsError(format!("S3 presigning error: {}", e)))?;

        Ok(presigned_request.uri().to_string())
    }

    /// Delete a save file
    pub async fn delete_save(
        &self,
        user_id: &str,
        game_id: &str,
        save_name: &str,
    ) -> Result<(), GameError> {
        // Get save metadata first
        let save = self.get_save_metadata(user_id, game_id, save_name).await?;

        // Delete from S3
        self.s3_client
            .delete_object()
            .bucket(&self.saves_bucket)
            .key(&save.s3_key)
            .send()
            .await
            .map_err(|e| GameError::AwsError(format!("S3 delete error: {}", e)))?;

        // Delete from DynamoDB
        self.dynamodb_client
            .delete_item()
            .table_name(&self.table_name)
            .key("PK", AttributeValue::S(format!("USER#{}", user_id)))
            .key("SK", AttributeValue::S(format!("SAVE#{}#{}", game_id, save_name)))
            .send()
            .await
            .map_err(|e| GameError::AwsError(format!("DynamoDB delete error: {}", e)))?;

        Ok(())
    }

    /// Get save metadata from DynamoDB
    async fn get_save_metadata(
        &self,
        user_id: &str,
        game_id: &str,
        save_name: &str,
    ) -> Result<SaveMetadata, GameError> {
        let result = self.dynamodb_client
            .get_item()
            .table_name(&self.table_name)
            .key("PK", AttributeValue::S(format!("USER#{}", user_id)))
            .key("SK", AttributeValue::S(format!("SAVE#{}#{}", game_id, save_name)))
            .send()
            .await
            .map_err(|e| GameError::AwsError(format!("DynamoDB error: {}", e)))?;

        let item = result.item.ok_or(GameError::SaveNotFound)?;
        Self::parse_save_metadata(item)
    }

    /// Parse DynamoDB item into SaveMetadata
    fn parse_save_metadata(
        item: std::collections::HashMap<String, AttributeValue>,
    ) -> Result<SaveMetadata, GameError> {
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

        Ok(SaveMetadata {
            user_id: get_str("user_id")?,
            game_id: get_str("game_id")?,
            save_name: get_str("save_name")?,
            s3_key: get_str("s3_key")?,
            file_size: get_num("file_size")? as u64,
            created_at: get_num("created_at")?,
            last_updated: get_num("last_updated")?,
        })
    }
}
