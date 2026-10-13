use serde::{Deserialize, Serialize};
use serde_json::Value;

/// User-chosen save folder, the last imported snapshot and the planner session
/// (which snapshot and club the web app has open).
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub folder: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_snapshot: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<Value>,
}
