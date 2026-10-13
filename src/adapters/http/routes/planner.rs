use crate::adapters::http::{
    app_state::Shared, error::ApiError, json_body::JsonBody, response::blocking,
};
use axum::{
    extract::{Path, State},
    response::Response,
    routing::{get, put},
    Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

pub fn routes() -> Router<Shared> {
    Router::new()
        .route(
            "/api/snapshots/{id}/planner/{key}",
            get(get_planner).put(put_planner),
        )
        .route("/api/snapshots/{id}/shortlist", get(get_shortlist))
        .route(
            "/api/snapshots/{id}/shortlist/{player}",
            put(add_shortlist).delete(remove_shortlist),
        )
        .route("/api/session", get(get_session).put(put_session))
}

#[derive(Deserialize)]
struct ValueBody {
    value: Value,
}

async fn get_planner(
    State(app): State<Shared>,
    Path((id, key)): Path<(String, String)>,
) -> Result<Response, ApiError> {
    blocking(app, 200, move |app| {
        Ok(json!({ "value": app.planner.get(&id, &key)? }))
    })
    .await
}
async fn put_planner(
    State(app): State<Shared>,
    Path((id, key)): Path<(String, String)>,
    JsonBody(body): JsonBody<ValueBody>,
) -> Result<Response, ApiError> {
    blocking(app, 200, move |app| {
        app.planner.put(&id, &key, &body.value)?;
        Ok(json!({ "value": body.value }))
    })
    .await
}
async fn get_shortlist(
    State(app): State<Shared>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    blocking(app, 200, move |app| {
        Ok(json!({ "ids": app.planner.shortlist(&id)? }))
    })
    .await
}
async fn add_shortlist(
    State(app): State<Shared>,
    Path((id, player)): Path<(String, u32)>,
) -> Result<Response, ApiError> {
    blocking(app, 200, move |app| {
        app.planner.shortlist_add(&id, player)?;
        Ok(json!({ "ok": true }))
    })
    .await
}
async fn remove_shortlist(
    State(app): State<Shared>,
    Path((id, player)): Path<(String, u32)>,
) -> Result<Response, ApiError> {
    blocking(app, 200, move |app| {
        app.planner.shortlist_remove(&id, player)?;
        Ok(json!({ "ok": true }))
    })
    .await
}
async fn get_session(State(app): State<Shared>) -> Result<Response, ApiError> {
    blocking(app, 200, |app| {
        Ok(json!({ "value": app.settings.session() }))
    })
    .await
}
async fn put_session(
    State(app): State<Shared>,
    JsonBody(body): JsonBody<ValueBody>,
) -> Result<Response, ApiError> {
    blocking(app, 200, move |app| {
        app.settings.set_session(body.value.clone())?;
        Ok(json!({ "value": body.value }))
    })
    .await
}
