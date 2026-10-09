use serde::{Deserialize, Serialize};

/// User-chosen save folder and the last imported snapshot.
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub folder: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_snapshot: Option<String>,
}
