use super::*;
use crate::utils::query::query_from_body;
use axum::{
    extract::{FromRequest, Path as UrlPath, RawQuery},
    routing::{get, post},
};

type Shared = Arc<AppState>;

pub fn json_response(status: u16, value: Value) -> Response {
    let mut response = (
        StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
        axum::Json(value),
    )
        .into_response();
    response
        .headers_mut()
        .insert("Cache-Control", "no-store".parse().unwrap());
    response
        .headers_mut()
        .insert("X-Content-Type-Options", "nosniff".parse().unwrap());
    response
}
pub fn error_response(e: Error) -> Response {
    let status = match e.code.as_str() {
        "INVALID_QUERY" => 400,
        "CONFLICT" => 409,
        "NOT_FOUND" => 404,
        "SHUTTING_DOWN" => 503,
        _ => 500,
    };
    let body = json!({"error":e.message});
    json_response(status, body)
}

/// A JSON object body, rejected with 400 (not axum's 415/422) to keep the API's error shape.
struct JsonBody(Value);
impl<S: Send + Sync> FromRequest<S> for JsonBody {
    type Rejection = Response;
    async fn from_request(req: Request<Body>, _: &S) -> std::result::Result<Self, Response> {
        let bad = |msg| json_response(400, json!({ "error": msg }));
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
            Ok(v) if v.is_object() => Ok(Self(v)),
            _ => Err(bad("Expected a JSON object.")),
        }
    }
}

/// Runs blocking work (SQLite, file I/O) off the async runtime and shapes the reply.
async fn run(
    app: Shared,
    status: u16,
    work: impl FnOnce(&AppState) -> Result<Value> + Send + 'static,
) -> Response {
    match tokio::task::spawn_blocking(move || work(&app)).await {
        Ok(Ok(v)) => json_response(status, v),
        Ok(Err(e)) => error_response(e),
        Err(_) => json_response(500, json!({"error":"Request failed."})),
    }
}

pub fn router() -> axum::Router<Shared> {
    axum::Router::new()
        .route("/api/health", get(health))
        .route("/api/attributes", get(attributes))
        .route("/api/settings", get(get_settings).put(put_settings))
        .route("/api/saves", get(saves))
        .route("/api/imports", post(start_import))
        .route("/api/imports/{id}", get(job).delete(cancel_import))
        .route("/api/snapshots/{id}", get(snapshot))
        .route("/api/snapshots/{id}/players", get(players))
        .route("/api/snapshots/{id}/players/search", post(search_players))
        .route("/api/snapshots/{id}/players/{player}", get(player))
}

async fn health() -> Response {
    json_response(
        200,
        json!({"app":"fm24-scout","productName":"FM SaveLens 24","version":VERSION,"parserVersion":PARSER_VERSION}),
    )
}
async fn attributes(State(app): State<Shared>) -> Response {
    run(app, 200, |_| Ok(serde_json::to_value(&*parser::CATALOG)?)).await
}
async fn get_settings(State(app): State<Shared>) -> Response {
    run(app, 200, |app| app.settings_json()).await
}
async fn put_settings(State(app): State<Shared>, JsonBody(body): JsonBody) -> Response {
    run(app, 200, move |app| app.set_folder(&body)).await
}
async fn saves(State(app): State<Shared>) -> Response {
    run(app, 200, |app| {
        let folder = app.inner.lock().unwrap().settings.folder.clone();
        Ok(json!({"saves": app.list(&folder)?}))
    })
    .await
}
async fn start_import(State(app): State<Shared>, JsonBody(body): JsonBody) -> Response {
    run(app.clone(), 202, move |_| {
        Ok(json!(app.start_import(body["saveId"].as_str().unwrap_or(""))?))
    })
    .await
}
async fn job(State(app): State<Shared>, UrlPath(id): UrlPath<String>) -> Response {
    run(app, 200, move |app| {
        let inner = app.inner.lock().unwrap();
        let job = inner.jobs.get(&id);
        job.map(|j| json!(j))
            .ok_or_else(|| Error::new("NOT_FOUND", "Import not found."))
    })
    .await
}
async fn cancel_import(State(app): State<Shared>, UrlPath(id): UrlPath<String>) -> Response {
    run(app, 200, move |app| Ok(json!(app.cancel(&id)?))).await
}
async fn snapshot(State(app): State<Shared>, UrlPath(id): UrlPath<String>) -> Response {
    run(app, 200, move |app| {
        storage::metadata(&storage::open_snapshot(&app.db_path(&id)?)?)
    })
    .await
}
async fn players(
    State(app): State<Shared>,
    UrlPath(id): UrlPath<String>,
    RawQuery(query): RawQuery,
) -> Response {
    run(app, 200, move |app| {
        let db = storage::open_snapshot(&app.db_path(&id)?)?;
        storage::search(&db, &query.unwrap_or_default())
    })
    .await
}
/// Same filters as the GET query string, sent as JSON to avoid URL length limits.
async fn search_players(
    State(app): State<Shared>,
    UrlPath(id): UrlPath<String>,
    JsonBody(body): JsonBody,
) -> Response {
    run(app, 200, move |app| {
        let db = storage::open_snapshot(&app.db_path(&id)?)?;
        storage::search(&db, &query_from_body(&body))
    })
    .await
}
async fn player(
    State(app): State<Shared>,
    UrlPath((id, player)): UrlPath<(String, String)>,
) -> Response {
    run(app, 200, move |app| {
        let player = player
            .parse()
            .map_err(|_| Error::new("NOT_FOUND", "Player not found."))?;
        storage::player(&storage::open_snapshot(&app.db_path(&id)?)?, player)
    })
    .await
}

impl AppState {
    fn settings_json(&self) -> Result<Value> {
        let inner = self.inner.lock().unwrap();
        let mut v = serde_json::to_value(&inner.settings)?;
        v["activeJob"] = inner
            .active
            .as_ref()
            .and_then(|a| inner.jobs.get(&a.id))
            .map_or(Value::Null, |j| json!(j));
        Ok(v)
    }
    fn set_folder(&self, body: &Value) -> Result<Value> {
        let mut inner = self.inner.lock().unwrap();
        if inner.active.is_some() {
            return Err(Error::query(
                "Wait for the current import before changing folders.",
            ));
        }
        let folder = body["folder"]
            .as_str()
            .filter(|s| s.len() <= 4096 && Path::new(s).is_dir())
            .ok_or_else(|| Error::query("Choose an existing save folder."))?;
        let settings = Settings {
            folder: canonical(Path::new(folder))?.to_string_lossy().into_owned(),
            last_snapshot: None,
        };
        self.persist(&settings)?;
        inner.settings = settings.clone();
        Ok(json!(settings))
    }
}
