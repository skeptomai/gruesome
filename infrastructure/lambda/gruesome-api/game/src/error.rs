use thiserror::Error;
use lambda_http::{Response, Body, http::StatusCode};

#[derive(Error, Debug)]
pub enum GameError {
    #[error("Game not found: {0}")]
    GameNotFound(String),

    #[error("Save not found")]
    SaveNotFound,

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("AWS error: {0}")]
    AwsError(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}

impl GameError {
    pub fn to_response(&self) -> Response<Body> {
        let (status, error_type) = match self {
            GameError::GameNotFound(_) => (StatusCode::NOT_FOUND, "game_not_found"),
            GameError::SaveNotFound => (StatusCode::NOT_FOUND, "save_not_found"),
            GameError::Unauthorized => (StatusCode::FORBIDDEN, "unauthorized"),
            GameError::InvalidRequest(_) => (StatusCode::BAD_REQUEST, "invalid_request"),
            GameError::AwsError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "aws_error"),
            GameError::InternalError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error"),
        };

        let body = serde_json::json!({
            "error": error_type,
            "message": self.to_string(),
        });

        Response::builder()
            .status(status)
            .header("Content-Type", "application/json")
            .body(body.to_string().into())
            .unwrap()
    }
}
