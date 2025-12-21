use lambda_http::{Body, Response};
use serde::Serialize;
use std::fmt;

#[derive(Debug)]
pub enum ApiError {
    BadRequest(String),
    Unauthorized,
    Forbidden(String),
    NotFound,
    InternalError(String),
    DynamoDbError(String),
    S3Error(String),
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ApiError::BadRequest(msg) => write!(f, "Bad Request: {}", msg),
            ApiError::Unauthorized => write!(f, "Unauthorized"),
            ApiError::Forbidden(msg) => write!(f, "Forbidden: {}", msg),
            ApiError::NotFound => write!(f, "Not Found"),
            ApiError::InternalError(msg) => write!(f, "Internal Error: {}", msg),
            ApiError::DynamoDbError(msg) => write!(f, "DynamoDB Error: {}", msg),
            ApiError::S3Error(msg) => write!(f, "S3 Error: {}", msg),
        }
    }
}

impl std::error::Error for ApiError {}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    message: String,
}

impl ApiError {
    pub fn into_response(self) -> Response<Body> {
        let (status, error_type, message) = match &self {
            ApiError::BadRequest(msg) => (400, "BadRequest", msg.clone()),
            ApiError::Unauthorized => (401, "Unauthorized", "Authentication required".to_string()),
            ApiError::Forbidden(msg) => (403, "Forbidden", msg.clone()),
            ApiError::NotFound => (404, "NotFound", "Resource not found".to_string()),
            ApiError::InternalError(msg) => (500, "InternalError", msg.clone()),
            ApiError::DynamoDbError(msg) => (500, "DynamoDbError", msg.clone()),
            ApiError::S3Error(msg) => (500, "S3Error", msg.clone()),
        };

        let error_response = ErrorResponse {
            error: error_type.to_string(),
            message,
        };

        let body = serde_json::to_string(&error_response).unwrap_or_else(|_| {
            r#"{"error":"InternalError","message":"Failed to serialize error"}"#.to_string()
        });

        Response::builder()
            .status(status)
            .header("Content-Type", "application/json")
            .body(Body::Text(body))
            .unwrap()
    }
}

impl From<aws_sdk_dynamodb::Error> for ApiError {
    fn from(err: aws_sdk_dynamodb::Error) -> Self {
        ApiError::DynamoDbError(format!("{:?}", err))
    }
}

impl From<aws_sdk_s3::Error> for ApiError {
    fn from(err: aws_sdk_s3::Error) -> Self {
        ApiError::S3Error(format!("{:?}", err))
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(err: serde_json::Error) -> Self {
        ApiError::BadRequest(format!("JSON parsing error: {}", err))
    }
}
