use super::{PlayerDetail, SnapshotMeta};
use std::collections::BTreeMap;

/// A player ready to be stored, with every derived value already worked out.
pub struct PlayerRecord {
    pub id: u32,
    pub uid: u32,
    pub name: String,
    pub full_name: String,
    /// Normalised names and uid, what text search matches against.
    pub search_name: String,
    pub age: i32,
    pub club_id: Option<u32>,
    pub club: Option<String>,
    pub nation_id: u16,
    pub nation_ids: Vec<u32>,
    pub nationalities: Vec<String>,
    pub positions: Vec<String>,
    pub position_mask: i64,
    pub main_position: usize,
    pub ca: u8,
    pub pa: u8,
    pub wage: Option<u32>,
    pub contract_end: Option<String>,
    pub value: Option<u32>,
    pub attributes: BTreeMap<String, u8>,
    pub detail: PlayerDetail,
}

/// An imported save ready to be stored.
pub struct NewSnapshot {
    pub meta: SnapshotMeta,
    pub players: Vec<PlayerRecord>,
}
