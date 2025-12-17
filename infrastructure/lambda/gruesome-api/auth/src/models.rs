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
    pub created_at: String,
}

impl UserRecord {
    pub fn new(user_id: String, email: String, username: String) -> Self {
        let display_name = username.clone();
        let created_at = chrono::Utc::now().to_rfc3339();

        Self {
            pk: format!("USER#{}", user_id),
            sk: "PROFILE".to_string(),
            user_id,
            email,
            username,
            display_name,
            created_at,
        }
    }

    pub fn to_profile(&self) -> UserProfile {
        UserProfile {
            user_id: self.user_id.clone(),
            email: self.email.clone(),
            username: self.username.clone(),
            display_name: self.display_name.clone(),
            created_at: self.created_at.clone(),
        }
    }
}
