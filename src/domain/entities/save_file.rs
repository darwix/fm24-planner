use serde::Serialize;

/// A `.fm` file found in the save folder.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveFile {
    pub id: String,
    pub name: String,
    pub size: u64,
    pub mtime: f64,
    pub modified: String,
    pub snapshot_id: String,
    pub cached: bool,
}
