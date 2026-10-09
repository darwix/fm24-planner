use super::response::reply;
use crate::Error;
use axum::response::{IntoResponse, Response};
use serde_json::json;

/// Wraps the application error so it can become an HTTP response.
pub struct ApiError(pub Error);

impl From<Error> for ApiError {
    fn from(e: Error) -> Self {
        Self(e)
    }
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match self.0.code.as_str() {
            "INVALID_QUERY" => 400,
            "CONFLICT" => 409,
            "NOT_FOUND" => 404,
            "SHUTTING_DOWN" => 503,
            _ => 500,
        };
        reply(status, json!({"error": self.0.message}))
    }
}
