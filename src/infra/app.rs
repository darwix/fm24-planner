use crate::services::{self, AppState};
use axum::{
    body::Body,
    extract::State,
    http::Request,
    middleware::{self, Next},
    response::Response,
    Router,
};
use serde_json::json;
use std::sync::Arc;

/// API routes behind the local-only guard. The web app is served by Vite, not by this server.
pub fn create_app(state: Arc<AppState>, port: u16) -> Router {
    services::routes::router()
        .with_state(state)
        .layer(middleware::from_fn_with_state(port, guard))
}

/// Local-only guard: the Host header and any Origin must be this server (or the Vite dev server).
async fn guard(State(port): State<u16>, req: Request<Body>, next: Next) -> Response {
    let host = req
        .headers()
        .get("host")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let allowed = [format!("127.0.0.1:{port}"), format!("localhost:{port}")];
    if !allowed.iter().any(|a| a == host) {
        return services::json_response(403, json!({"error":"Invalid host."}));
    }
    if let Some(origin) = req.headers().get("origin") {
        let origin = origin.to_str().unwrap_or("");
        if origin != "http://127.0.0.1:5173"
            && !allowed.iter().any(|h| origin == format!("http://{h}"))
        {
            return services::json_response(
                403,
                json!({"error":"This app accepts local workspace requests only."}),
            );
        }
    }
    next.run(req).await
}
