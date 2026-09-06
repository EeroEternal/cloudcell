use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("{0}")]
    BadRequest(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("{0}")]
    Forbidden(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("{0}")]
    Conflict(String),

    #[error("Not implemented: {0}")]
    NotImplemented(String),

    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            Error::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            Error::Conflict(msg) => (StatusCode::CONFLICT, msg.clone()),
            Error::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            Error::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg.clone()),
            Error::Forbidden(msg) => (StatusCode::FORBIDDEN, msg.clone()),
            Error::NotImplemented(msg) => (StatusCode::NOT_IMPLEMENTED, msg.clone()),
            Error::Database(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
            Error::Config(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            Error::Internal(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
        };

        let body = Json(json!({
            "error": {
                "message": message,
                "code": status.as_u16(),
            }
        }));

        (status, body).into_response()
    }
}
