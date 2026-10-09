use serde::Serialize;

/// One import run, as the API reports it.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub id: String,
    pub save_id: String,
    pub snapshot_id: String,
    pub status: String,
    pub progress: u8,
    pub message: String,
    pub started_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub player_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elapsed_ms: Option<u128>,
}
