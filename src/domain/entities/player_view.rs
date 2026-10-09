use super::Player;
use serde::{Deserialize, Serialize};

/// One row of a player search.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerSummary {
    pub id: u32,
    pub uid: u32,
    pub name: String,
    pub age: i32,
    #[serde(rename = "club_id")]
    pub club_id: Option<u32>,
    pub club: Option<String>,
    pub nationalities: Vec<String>,
    pub positions: Vec<String>,
    pub ca: u8,
    pub pa: u8,
    pub wage: Option<u32>,
    pub contract_end: Option<String>,
    pub value: Option<u32>,
}

/// One page of search results.
#[derive(Serialize)]
pub struct PlayerPage {
    pub total: i64,
    pub page: i64,
    pub limit: i64,
    pub players: Vec<PlayerSummary>,
}

/// A player with the names of their nationalities, as the player page shows.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerDetail {
    #[serde(flatten)]
    pub player: Player,
    pub nationalities: Vec<String>,
}
