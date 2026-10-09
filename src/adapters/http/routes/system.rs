use crate::{
    adapters::{
        http::{
            app_state::Shared,
            dto::system::{FolderBody, HealthResponse, SettingsResponse},
            error::ApiError,
            json_body::JsonBody,
            response::{blocking, reply},
        },
        parser::CATALOG,
    },
    VERSION,
};
use axum::{extract::State, response::Response, routing::get, Router};

pub fn routes() -> Router<Shared> {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/attributes", get(attributes))
        .route("/api/settings", get(get_settings).put(put_settings))
}

async fn health(State(app): State<Shared>) -> Response {
    reply(
        200,
        HealthResponse {
            app: "fm24-scout",
            product_name: "FM SaveLens 24",
            version: VERSION,
            parser_version: app.saves.parser_version().to_owned(),
        },
    )
}
async fn attributes() -> Response {
    reply(200, &*CATALOG)
}
async fn get_settings(State(app): State<Shared>) -> Result<Response, ApiError> {
    blocking(app, 200, |app| {
        let (settings, active_job) = app.settings.settings();
        Ok(SettingsResponse {
            settings,
            active_job,
        })
    })
    .await
}
async fn put_settings(
    State(app): State<Shared>,
    JsonBody(body): JsonBody<FolderBody>,
) -> Result<Response, ApiError> {
    blocking(app, 200, move |app| {
        app.settings.set_folder(body.folder.as_deref())
    })
    .await
}
