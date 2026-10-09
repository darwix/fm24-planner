use crate::adapters::http::{
    app_state::Shared, dto::saves::SavesResponse, error::ApiError, response::blocking,
};
use axum::{extract::State, response::Response, routing::get, Router};

pub fn routes() -> Router<Shared> {
    Router::new().route("/api/saves", get(list_saves))
}

async fn list_saves(State(app): State<Shared>) -> Result<Response, ApiError> {
    blocking(app, 200, |app| {
        Ok(SavesResponse {
            saves: app.saves.list_saves()?,
        })
    })
    .await
}
