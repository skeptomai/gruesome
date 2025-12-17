use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    pub sub: String,        // Subject (user ID)
    pub exp: usize,         // Expiration time
    pub iat: usize,         // Issued at
    pub token_use: String,  // "access" or "id"
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

    /// Validate JWT token and return user ID
    /// Returns user_id (sub claim) on success
    pub fn validate_token(&self, token: &str) -> Result<String, String> {
        // Decode header to check algorithm
        let header = decode_header(token)
            .map_err(|e| format!("Failed to decode JWT header: {:?}", e))?;

        if header.alg != Algorithm::RS256 {
            return Err(format!("Unsupported algorithm: {:?}", header.alg));
        }

        // In production, we would fetch Cognito's public keys (JWKS) and verify signature
        // For development, we'll use insecure validation to decode claims
        // TODO: Implement proper signature verification with Cognito JWKS

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

        let token_data = decode::<Claims>(token, &decoding_key, &validation)
            .map_err(|e| format!("Failed to decode JWT: {:?}", e))?;

        Ok(token_data.claims.sub)
    }
}
