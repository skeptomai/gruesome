use aws_sdk_dynamodb::{
    types::AttributeValue,
    Client,
};
use std::collections::HashMap;
use tracing::{error, info};

use crate::error::{AuthError, AuthResult};
use crate::models::{UserProfile, UserRecord};

pub struct DynamoDbService {
    client: Client,
    table_name: String,
}

impl DynamoDbService {
    pub fn new(client: Client, table_name: String) -> Self {
        Self { client, table_name }
    }

    /// Create a new user record
    pub async fn create_user(&self, record: &UserRecord) -> AuthResult<()> {
        info!("Creating user record for: {}", record.user_id);

        let mut item = HashMap::new();
        item.insert("PK".to_string(), AttributeValue::S(record.pk.clone()));
        item.insert("SK".to_string(), AttributeValue::S(record.sk.clone()));
        item.insert(
            "user_id".to_string(),
            AttributeValue::S(record.user_id.clone()),
        );
        item.insert("email".to_string(), AttributeValue::S(record.email.clone()));
        item.insert(
            "username".to_string(),
            AttributeValue::S(record.username.clone()),
        );
        item.insert(
            "display_name".to_string(),
            AttributeValue::S(record.display_name.clone()),
        );
        item.insert(
            "created_at".to_string(),
            AttributeValue::N(record.created_at.to_string()),
        );
        item.insert(
            "entity_type".to_string(),
            AttributeValue::S(record.entity_type.clone()),
        );

        self.client
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(item))
            .send()
            .await
            .map_err(|e| {
                error!("DynamoDB PutItem error: {:?}", e);
                AuthError::DynamoDbError(e.to_string())
            })?;

        info!("User record created successfully");
        Ok(())
    }

    /// Get user profile by user ID
    pub async fn get_user(&self, user_id: &str) -> AuthResult<UserProfile> {
        info!("Getting user profile for: {}", user_id);

        let pk = format!("USER#{}", user_id);
        let sk = "PROFILE";

        let result = self
            .client
            .get_item()
            .table_name(&self.table_name)
            .key("PK", AttributeValue::S(pk.clone()))
            .key("SK", AttributeValue::S(sk.to_string()))
            .send()
            .await
            .map_err(|e| {
                error!("DynamoDB GetItem error: {:?}", e);
                AuthError::DynamoDbError(e.to_string())
            })?;

        let item = result
            .item()
            .ok_or_else(|| AuthError::UserNotFound)?;

        let user_id = get_string_attr(item, "user_id")?;
        let email = get_string_attr(item, "email")?;
        let username = get_string_attr(item, "username")?;
        let display_name = get_string_attr(item, "display_name")?;

        // Get created_at as number and convert to RFC3339 string
        let created_at_timestamp = get_number_attr(item, "created_at")?;
        let created_at = chrono::DateTime::from_timestamp(created_at_timestamp, 0)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_else(|| "unknown".to_string());

        Ok(UserProfile {
            user_id,
            email,
            username,
            display_name,
            created_at,
        })
    }

    /// Update user profile
    pub async fn update_user(&self, user_id: &str, display_name: &str) -> AuthResult<()> {
        info!("Updating user profile for: {}", user_id);

        let pk = format!("USER#{}", user_id);
        let sk = "PROFILE";

        self.client
            .update_item()
            .table_name(&self.table_name)
            .key("PK", AttributeValue::S(pk))
            .key("SK", AttributeValue::S(sk.to_string()))
            .update_expression("SET display_name = :display_name")
            .expression_attribute_values(
                ":display_name",
                AttributeValue::S(display_name.to_string()),
            )
            .send()
            .await
            .map_err(|e| {
                error!("DynamoDB UpdateItem error: {:?}", e);
                AuthError::DynamoDbError(e.to_string())
            })?;

        info!("User profile updated successfully");
        Ok(())
    }
}

/// Helper function to extract string attribute from DynamoDB item
fn get_string_attr(item: &HashMap<String, AttributeValue>, key: &str) -> AuthResult<String> {
    item.get(key)
        .and_then(|v| v.as_s().ok())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            AuthError::InternalError(format!("Missing or invalid attribute: {}", key))
        })
}

/// Helper function to extract number attribute from DynamoDB item
fn get_number_attr(item: &HashMap<String, AttributeValue>, key: &str) -> AuthResult<i64> {
    item.get(key)
        .and_then(|v| v.as_n().ok())
        .and_then(|s| s.parse::<i64>().ok())
        .ok_or_else(|| {
            AuthError::InternalError(format!("Missing or invalid number attribute: {}", key))
        })
}
