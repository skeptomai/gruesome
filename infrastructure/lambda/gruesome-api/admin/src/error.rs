use lambda_http::{Body, Response};
use thiserror::Error;

pub type AdminResult<T> = Result<T, AdminError>;

#[derive(Error, Debug)]
pub enum AdminError {
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("DynamoDB error: {0}")]
    DynamoDbError(String),

    #[error("S3 error: {0}")]
    S3Error(String),

    #[error("JWT validation error: {0}")]
    JwtError(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}

impl AdminError {
    pub fn to_response(&self) -> Response<Body> {
        let (status, error_type) = match self {
            AdminError::InvalidRequest(_) => (400, "invalid_request"),
            AdminError::Unauthorized(_) => (401, "unauthorized"),
            AdminError::Forbidden(_) => (403, "forbidden"),
            AdminError::NotFound(_) => (404, "not_found"),
            AdminError::DynamoDbError(_) => (500, "database_error"),
            AdminError::S3Error(_) => (500, "storage_error"),
            AdminError::JwtError(_) => (401, "invalid_token"),
            AdminError::InternalError(_) => (500, "internal_error"),
        };

        let error_body = serde_json::json!({
            "error": error_type,
            "message": self.to_string()
        });

        Response::builder()
            .status(status)
            .header("Content-Type", "application/json")
            .body(error_body.to_string().into())
            .unwrap()
    }
}
