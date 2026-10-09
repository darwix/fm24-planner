use crate::adapters::http::{
    app_state::Shared,
    dto::player_query::{parse_filter, query_from_body},
    error::ApiError,
    json_body::JsonBody,
    response::blocking,
};
use axum::{
    extract::{Path, RawQuery, State},
    response::Response,
    routing::{get, post},
    Router,
};
use serde_json::Value;

pub fn routes() -> Router<Shared> {
    Router::new()
        .route("/api/snapshots/{id}", get(get_snapshot))
        .route("/api/snapshots/{id}/players", get(search_players))
        .route(
            "/api/snapshots/{id}/players/search",
            post(search_players_body),
        )
        .route("/api/snapshots/{id}/players/{player}", get(get_player))
}

async fn get_snapshot(
    State(app): State<Shared>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    blocking(app, 200, move |app| app.saves.snapshot(&id)).await
}
async fn search_players(
    State(app): State<Shared>,
    Path(id): Path<String>,
    RawQuery(query): RawQuery,
) -> Result<Response, ApiError> {
    blocking(app, 200, move |app| {
        app.players
            .search_players(&id, &parse_filter(&query.unwrap_or_default())?)
    })
    .await
}
/// Same filters as the GET query string, sent as JSON to avoid URL length limits.
async fn search_players_body(
    State(app): State<Shared>,
    Path(id): Path<String>,
    JsonBody(body): JsonBody<Value>,
) -> Result<Response, ApiError> {
    blocking(app, 200, move |app| {
        app.players
            .search_players(&id, &parse_filter(&query_from_body(&body))?)
    })
    .await
}
async fn get_player(
    State(app): State<Shared>,
    Path((id, player)): Path<(String, String)>,
) -> Result<Response, ApiError> {
    blocking(app, 200, move |app| app.players.player(&id, &player)).await
}
