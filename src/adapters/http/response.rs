use super::{app_state::Shared, error::ApiError};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

/// JSON reply with the no-cache and no-sniff headers every API response carries.
pub fn reply(status: u16, body: impl Serialize) -> Response {
    let mut response = (
        StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
        axum::Json(body),
    )
        .into_response();
    let headers = response.headers_mut();
    headers.insert("Cache-Control", "no-store".parse().unwrap());
    headers.insert("X-Content-Type-Options", "nosniff".parse().unwrap());
    response
}

/// Runs blocking work (SQLite, file I/O) off the async runtime and shapes the reply.
pub async fn blocking<T: Serialize + Send + 'static>(
    app: Shared,
    status: u16,
    work: impl FnOnce(&Shared) -> crate::Result<T> + Send + 'static,
) -> Result<Response, ApiError> {
    match tokio::task::spawn_blocking(move || work(&app)).await {
        Ok(result) => Ok(reply(status, result?)),
        Err(_) => Err(crate::Error::new("INTERNAL", "Request failed.").into()),
    }
}
