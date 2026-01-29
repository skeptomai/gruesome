use lambda_http::{Body, Response};
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("User already exists")]
    UserAlreadyExists,

    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("User not found")]
    UserNotFound,

    #[error("Invalid token")]
    InvalidToken,

    #[error("Token expired")]
    TokenExpired,

    #[error("Email not verified")]
    EmailNotVerified,

    #[error("Cognito error: {0}")]
    CognitoError(String),

    #[error("DynamoDB error: {0}")]
    DynamoDbError(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

impl AuthError {
    pub fn to_response(&self) -> Response<Body> {
        let (status, error_type) = match self {
            AuthError::InvalidRequest(_) => (400, "invalid_request"),
            AuthError::UserAlreadyExists => (409, "user_exists"),
            AuthError::InvalidCredentials => (401, "invalid_credentials"),
            AuthError::UserNotFound => (404, "user_not_found"),
            AuthError::InvalidToken => (401, "invalid_token"),
            AuthError::TokenExpired => (401, "token_expired"),
            AuthError::EmailNotVerified => (403, "email_not_verified"),
            AuthError::CognitoError(_) => (500, "cognito_error"),
            AuthError::DynamoDbError(_) => (500, "dynamodb_error"),
            AuthError::InternalError(_) => (500, "internal_error"),
        };

        let error_response = ErrorResponse {
            error: error_type.to_string(),
            message: self.to_string(),
            details: match self {
                AuthError::InvalidRequest(msg) => Some(msg.clone()),
                AuthError::CognitoError(msg) => Some(msg.clone()),
                AuthError::DynamoDbError(msg) => Some(msg.clone()),
                AuthError::InternalError(msg) => Some(msg.clone()),
                _ => None,
            },
        };

        Response::builder()
            .status(status)
            .header("Content-Type", "application/json")
            .body(
                serde_json::to_string(&error_response)
                    .unwrap_or_else(|_| r#"{"error":"serialization_error"}"#.to_string())
                    .into(),
            )
            .unwrap()
    }
}

pub type AuthResult<T> = Result<T, AuthError>;
