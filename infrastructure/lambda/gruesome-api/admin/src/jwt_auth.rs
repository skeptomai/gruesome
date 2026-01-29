use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use crate::error::{AdminError, AdminResult};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // Subject (user ID)
    #[serde(default)]
    pub email: Option<String>, // Email (only in ID tokens)
    pub exp: usize,  // Expiration time
    pub iat: usize,  // Issued at
    pub token_use: String, // "access" or "id"
    #[serde(default, rename = "cognito:username")]
    pub cognito_username: Option<String>, // Username (Cognito-specific)
    #[serde(default)]
    pub username: Option<String>, // Username (in access tokens)
}

pub struct JwtValidator {
    region: String,
    user_pool_id: String,
}

impl JwtValidator {
    pub fn new(region: String, user_pool_id: String) -> Self {
        Self {
            region,
            user_pool_id,
        }
    }

    /// Extract token from Authorization header
    pub fn extract_token(auth_header: &str) -> AdminResult<String> {
        if !auth_header.starts_with("Bearer ") {
            return Err(AdminError::Unauthorized("Missing Bearer token".to_string()));
        }

        let token = auth_header.trim_start_matches("Bearer ").trim();
        if token.is_empty() {
            return Err(AdminError::Unauthorized("Empty token".to_string()));
        }

        Ok(token.to_string())
    }

    /// Validate JWT token (simplified - in production, verify signature with Cognito public keys)
    pub async fn validate_token(&self, token: &str) -> AdminResult<Claims> {
        info!("Validating JWT token");

        // Decode header to check algorithm
        let header = decode_header(token).map_err(|e| {
            error!("Failed to decode JWT header: {:?}", e);
            AdminError::JwtError("Invalid token format".to_string())
        })?;

        if header.alg != Algorithm::RS256 {
            error!("Unsupported algorithm: {:?}", header.alg);
            return Err(AdminError::JwtError("Unsupported algorithm".to_string()));
        }

        // In production, we would fetch Cognito's public keys (JWKS) and verify signature
        // For development, we'll use insecure validation to decode claims
        let mut validation = Validation::new(Algorithm::RS256);
        validation.insecure_disable_signature_validation(); // DEVELOPMENT ONLY!
        validation.validate_exp = true;

        // Set expected issuer (Cognito User Pool)
        let issuer = format!(
            "https://cognito-idp.{}.amazonaws.com/{}",
            self.region, self.user_pool_id
        );
        validation.set_issuer(&[issuer]);

        // For development, use a dummy key since we disabled signature validation
        let decoding_key = DecodingKey::from_secret(&[]);

        let token_data = decode::<Claims>(token, &decoding_key, &validation).map_err(|e| {
            error!("Failed to decode JWT: {:?}", e);
            match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                    AdminError::Unauthorized("Token expired".to_string())
                }
                _ => AdminError::JwtError("Invalid token".to_string()),
            }
        })?;

        info!(
            "Token validated successfully for user: {}",
            token_data.claims.sub
        );
        Ok(token_data.claims)
    }

    /// Get user ID from token
    pub async fn get_user_id_from_token(&self, auth_header: &str) -> AdminResult<String> {
        let token = Self::extract_token(auth_header)?;
        let claims = self.validate_token(&token).await?;
        Ok(claims.sub)
    }
}
