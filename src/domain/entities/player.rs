use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// One player as read from the save.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Player {
    pub id: u32,
    pub uid: u32,
    pub name: String,
    pub full_name: String,
    pub age: i32,
    pub birth_date: String,
    pub nation_id: u16,
    pub other_nation_ids: Vec<u32>,
    pub club_id: Option<u32>,
    pub club: Option<String>,
    pub ca: u8,
    pub pa: u8,
    pub positions: Vec<String>,
    pub position_ratings: Vec<u8>,
    pub attributes: BTreeMap<String, u8>,
    pub raw_attributes: Vec<u8>,
    pub wage: Option<u32>,
    pub contract_end: Option<String>,
    pub value: Option<u32>,
    pub source: Value,
}
