use aws_sdk_dynamodb::types::AttributeValue;
use aws_sdk_dynamodb::Client as DynamoDbClient;
use lambda_http::Request;
use std::collections::HashMap;
use tracing::{info, warn};

use crate::error::ApiError;

/// Extract user ID from JWT token in Authorization header
///
/// Expected format: "Bearer <jwt_token>"
/// The JWT contains claims including 'sub' which is the user_id
pub fn extract_user_id_from_request(event: &Request) -> Result<String, ApiError> {
    // Get Authorization header
    let auth_header = event
        .headers()
        .get("authorization")
        .or_else(|| event.headers().get("Authorization"))
        .ok_or(ApiError::Unauthorized)?;

    let auth_str = auth_header
        .to_str()
        .map_err(|_| ApiError::Unauthorized)?;

    // Extract token from "Bearer <token>"
    if !auth_str.starts_with("Bearer ") {
        return Err(ApiError::Unauthorized);
    }

    let token = auth_str.trim_start_matches("Bearer ").trim();

    // Parse JWT to extract user_id from 'sub' claim
    // Note: In production, you should verify the JWT signature
    // For now, we'll do basic parsing to extract the 'sub' claim
    let user_id = parse_jwt_sub_claim(token)?;

    info!("Extracted user_id from token: {}", user_id);

    Ok(user_id)
}

/// Parse JWT token to extract 'sub' claim (user_id)
///
/// Note: This is a simplified implementation that doesn't verify signature.
/// In production, use a proper JWT library to verify tokens.
fn parse_jwt_sub_claim(token: &str) -> Result<String, ApiError> {
    // JWT format: header.payload.signature
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err(ApiError::Unauthorized);
    }

    // Decode payload (base64url)
    let payload = parts[1];
    let decoded = base64_decode_url(payload).map_err(|_| ApiError::Unauthorized)?;

    // Parse JSON
    let payload_json: HashMap<String, serde_json::Value> =
        serde_json::from_slice(&decoded).map_err(|_| ApiError::Unauthorized)?;

    // Extract 'sub' claim
    let user_id = payload_json
        .get("sub")
        .and_then(|v| v.as_str())
        .ok_or(ApiError::Unauthorized)?
        .to_string();

    Ok(user_id)
}

/// Base64 URL decode (without padding)
fn base64_decode_url(input: &str) -> Result<Vec<u8>, String> {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine;

    URL_SAFE_NO_PAD
        .decode(input)
        .map_err(|e| format!("Base64 decode error: {}", e))
}

/// Verify that the user has admin role
///
/// Checks DynamoDB USER#{user_id}/PROFILE for role="admin"
pub async fn verify_admin_role(
    user_id: &str,
    dynamodb: &DynamoDbClient,
    table_name: &str,
) -> Result<(), ApiError> {
    info!("Verifying admin role for user: {}", user_id);

    // Query DynamoDB for user profile
    let result = dynamodb
        .get_item()
        .table_name(table_name)
        .key("PK", AttributeValue::S(format!("USER#{}", user_id)))
        .key("SK", AttributeValue::S("PROFILE".to_string()))
        .send()
        .await
        .map_err(|e| {
            warn!("DynamoDB error getting user profile: {:?}", e);
            ApiError::DynamoDbError(format!("Failed to get user profile: {:?}", e))
        })?;

    // Check if user exists
    let item = result.item().ok_or_else(|| {
        warn!("User not found: {}", user_id);
        ApiError::Forbidden("User not found".to_string())
    })?;

    // Extract role field
    let role = item
        .get("role")
        .and_then(|v| v.as_s().ok().map(|s| s.as_str()))
        .unwrap_or("user"); // Default to "user" if role field missing

    info!("User {} has role: {}", user_id, role);

    // Verify admin role
    if role != "admin" {
        warn!("User {} attempted admin access with role: {}", user_id, role);
        return Err(ApiError::Forbidden(
            "Admin access required. Contact an administrator to request admin privileges."
                .to_string(),
        ));
    }

    info!("Admin access granted for user: {}", user_id);
    Ok(())
}

/// Helper function to verify admin and return user_id
pub async fn require_admin(
    event: &Request,
    dynamodb: &DynamoDbClient,
    table_name: &str,
) -> Result<String, ApiError> {
    let user_id = extract_user_id_from_request(event)?;
    verify_admin_role(&user_id, dynamodb, table_name).await?;
    Ok(user_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_decode_url() {
        // Test with a known base64url encoded string
        let input = "eyJzdWIiOiIxMjM0NTYifQ"; // {"sub":"123456"}
        let result = base64_decode_url(input).unwrap();
        let decoded = String::from_utf8(result).unwrap();
        assert!(decoded.contains("sub"));
        assert!(decoded.contains("123456"));
    }
}
