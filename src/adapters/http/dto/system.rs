use crate::domain::entities::{Job, Settings};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthResponse {
    pub app: &'static str,
    pub product_name: &'static str,
    pub version: &'static str,
    pub parser_version: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsResponse {
    #[serde(flatten)]
    pub settings: Settings,
    pub active_job: Option<Job>,
}

#[derive(Deserialize)]
pub struct FolderBody {
    pub folder: Option<String>,
}
