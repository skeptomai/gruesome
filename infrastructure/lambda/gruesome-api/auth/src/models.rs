use serde::{Deserialize, Serialize};

// ============================================================================
// Signup
// ============================================================================

#[derive(Deserialize, Debug)]
pub struct SignupRequest {
    pub email: String,
    pub password: String,
    pub username: String,
}

#[derive(Serialize)]
pub struct SignupResponse {
    pub user_id: String,
    pub email: String,
    pub username: String,
    pub message: String,
}

// ============================================================================
// Login
// ============================================================================

#[derive(Deserialize, Debug)]
pub struct LoginRequest {
    pub username: String,  // USER_PASSWORD_AUTH requires username, not email
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub id_token: String,
    pub expires_in: i32,
    pub token_type: String,
}

// ============================================================================
// Refresh Token
// ============================================================================

#[derive(Deserialize, Debug)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Serialize)]
pub struct RefreshResponse {
    pub access_token: String,
    pub id_token: String,
    pub expires_in: i32,
    pub token_type: String,
}

// ============================================================================
// Get User Profile
// ============================================================================

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserProfile {
    pub user_id: String,
    pub email: String,
    pub username: String,
    pub display_name: String,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct MeResponse {
    pub profile: UserProfile,
}

// ============================================================================
// Password Reset
// ============================================================================

#[derive(Deserialize, Debug)]
pub struct ForgotPasswordRequest {
    pub username: String,
}

#[derive(Serialize)]
pub struct ForgotPasswordResponse {
    pub message: String,
}

#[derive(Deserialize, Debug)]
pub struct ConfirmForgotPasswordRequest {
    pub username: String,
    pub confirmation_code: String,
    pub new_password: String,
}

#[derive(Serialize)]
pub struct ConfirmForgotPasswordResponse {
    pub message: String,
}

// ============================================================================
// DynamoDB User Record
// ============================================================================

#[derive(Serialize, Deserialize, Debug)]
pub struct UserRecord {
    #[serde(rename = "PK")]
    pub pk: String, // USER#<user_id>
    #[serde(rename = "SK")]
    pub sk: String, // PROFILE
    pub user_id: String,
    pub email: String,
    pub username: String,
    pub display_name: String,
    pub created_at: i64, // Unix timestamp for DynamoDB GSI
    pub entity_type: String, // Required for GSI
}

impl UserRecord {
    pub fn new(user_id: String, email: String, username: String) -> Self {
        let display_name = username.clone();
        let created_at = chrono::Utc::now().timestamp();

        Self {
            pk: format!("USER#{}", user_id),
            sk: "PROFILE".to_string(),
            user_id,
            email,
            username,
            display_name,
            created_at,
            entity_type: "USER".to_string(),
        }
    }

    pub fn to_profile(&self) -> UserProfile {
        // Convert Unix timestamp to RFC3339 string for API response
        let created_at_str = chrono::DateTime::from_timestamp(self.created_at, 0)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_else(|| "unknown".to_string());

        UserProfile {
            user_id: self.user_id.clone(),
            email: self.email.clone(),
            username: self.username.clone(),
            display_name: self.display_name.clone(),
            created_at: created_at_str,
        }
    }
}
