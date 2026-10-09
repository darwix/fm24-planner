use crate::adapters::http::{
    app_state::Shared, dto::saves::ImportBody, error::ApiError, json_body::JsonBody,
    response::blocking,
};
use axum::{
    extract::{Path, State},
    response::Response,
    routing::{get, post},
    Router,
};

pub fn routes() -> Router<Shared> {
    Router::new()
        .route("/api/imports", post(start_import))
        .route("/api/imports/{id}", get(get_import).delete(cancel_import))
}

async fn start_import(
    State(app): State<Shared>,
    JsonBody(body): JsonBody<ImportBody>,
) -> Result<Response, ApiError> {
    blocking(app, 202, move |app| app.saves.start_import(&body.save_id)).await
}
async fn get_import(
    State(app): State<Shared>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    blocking(app, 200, move |app| app.saves.job(&id)).await
}
async fn cancel_import(
    State(app): State<Shared>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    blocking(app, 200, move |app| app.saves.cancel(&id)).await
}
