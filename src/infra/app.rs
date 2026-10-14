use crate::adapters::http::{app_state::Shared, response::reply, routes::router};
use axum::{
    body::Body,
    extract::State,
    http::Request,
    middleware::{self, Next},
    response::Response,
    Router,
};
use serde_json::json;

/// API routes behind the local-only guard. The web app is served by Vite, not by this server.
pub fn create_app(state: Shared, port: u16) -> Router {
    router()
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
        return reply(403, json!({"error":"Invalid host."}));
    }
    if let Some(origin) = req.headers().get("origin") {
        let origin = origin.to_str().unwrap_or("");
        if origin != "http://127.0.0.1:5173"
            && !allowed.iter().any(|h| origin == format!("http://{h}"))
        {
            return reply(
                403,
                json!({"error":"This app accepts local workspace requests only."}),
            );
        }
    }
    next.run(req).await
}

/// Serve the built web app from `dir` (the desktop app), falling back to `index.html` for routes.
pub fn serve_web(router: Router, dir: std::path::PathBuf) -> Router {
    router.fallback(move |req: Request<Body>| {
        let dir = dir.clone();
        async move {
            let rel = req.uri().path().trim_start_matches('/');
            // ponytail: no percent-decoding; Vite's build output has plain ASCII names
            let mut path = dir.join(rel);
            if rel.starts_with("api/") {
                return reply(404, json!({"error":"Not found."}));
            }
            if rel.contains("..") || !path.is_file() {
                path = dir.join("index.html");
            }
            let mime = match path.extension().and_then(|e| e.to_str()) {
                Some("html") => "text/html; charset=utf-8",
                Some("js") => "text/javascript",
                Some("css") => "text/css",
                Some("svg") => "image/svg+xml",
                Some("png") => "image/png",
                _ => "application/octet-stream",
            };
            match std::fs::read(&path) {
                Ok(bytes) => Response::builder()
                    .header("content-type", mime)
                    .body(Body::from(bytes))
                    .unwrap(),
                Err(_) => reply(404, json!({"error":"Not found."})),
            }
        }
    })
}
