use super::response::reply;
use axum::{
    body::{to_bytes, Body},
    extract::FromRequest,
    http::Request,
    response::Response,
};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};

/// A JSON object body, rejected with 400 (not axum's 415/422) to keep the API's error shape.
pub struct JsonBody<T>(pub T);

impl<S: Send + Sync, T: DeserializeOwned> FromRequest<S> for JsonBody<T> {
    type Rejection = Response;
    async fn from_request(req: Request<Body>, _: &S) -> Result<Self, Response> {
        let bad = |msg| reply(400, json!({ "error": msg }));
        let is_json = req
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .is_some_and(|s| s.starts_with("application/json"));
        if !is_json {
            return Err(bad("Expected JSON."));
        }
        let bytes = to_bytes(req.into_body(), 256 * 1024)
            .await
            .map_err(|_| bad("Request is too large."))?;
        match serde_json::from_slice::<Value>(&bytes) {
            Ok(v) if v.is_object() => serde_json::from_value(v)
                .map(Self)
                .map_err(|_| bad("Invalid request body.")),
            _ => Err(bad("Expected a JSON object.")),
        }
    }
}
