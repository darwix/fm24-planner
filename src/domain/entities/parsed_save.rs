use super::{Club, Player};
use serde::Serialize;
use serde_json::Value;

/// Everything read from one save file.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedSave {
    pub name: String,
    pub format_version: String,
    pub game_date: String,
    pub players: Vec<Player>,
    pub clubs: Vec<Club>,
    pub managed_club_id: Option<u32>,
    pub diagnostics: Value,
    pub warnings: Vec<String>,
}
