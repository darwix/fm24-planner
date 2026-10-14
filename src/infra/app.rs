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

/// Serve the built web app through `get(path) -> (bytes, mime)` (the desktop app's embedded
/// assets), falling back to `index.html` for routes.
pub fn serve_web<F>(router: Router, get: F) -> Router
where
    F: Fn(&str) -> Option<(Vec<u8>, String)> + Clone + Send + Sync + 'static,
{
    router.fallback(move |req: Request<Body>| {
        let get = get.clone();
        async move {
            let rel = req.uri().path().trim_start_matches('/');
            if rel.starts_with("api/") {
                return reply(404, json!({"error":"Not found."}));
            }
            match get(rel).or_else(|| get("index.html")) {
                Some((bytes, mime)) => Response::builder()
                    .header("content-type", mime)
                    .body(Body::from(bytes))
                    .unwrap(),
                None => reply(404, json!({"error":"Not found."})),
            }
        }
    })
}
