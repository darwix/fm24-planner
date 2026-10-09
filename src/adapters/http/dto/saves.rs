use crate::domain::entities::SaveFile;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct SavesResponse {
    pub saves: Vec<SaveFile>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportBody {
    #[serde(default)]
    pub save_id: String,
}
