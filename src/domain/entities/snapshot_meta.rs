use serde::{Deserialize, Serialize};
use serde_json::Value;

/// An id with a display name, for the nation and club lists of a snapshot.
#[derive(Clone, Serialize, Deserialize)]
pub struct NamedId {
    pub id: u32,
    pub name: String,
}

/// The summary of an imported save, stored with its snapshot. `stale` is not
/// stored: it is worked out each time the snapshot is read.
#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SnapshotMeta {
    pub snapshot_id: String,
    pub source_path: String,
    pub source_name: String,
    pub source_size: u64,
    pub source_mtime: f64,
    pub parser_version: String,
    pub name: String,
    pub game_date: String,
    pub format_version: String,
    pub player_count: usize,
    pub diagnostics: Value,
    pub warnings: Vec<String>,
    pub imported_at: String,
    pub nations: Vec<NamedId>,
    pub clubs: Vec<NamedId>,
    pub managed_club_id: Option<u32>,
    pub stale: bool,
}
