use super::{NamedId, ParsedSave, PlayerDetail, SnapshotMeta};
use crate::domain::{countries::nation_name, text::normalize};
use std::collections::{BTreeMap, BTreeSet};

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

impl NewSnapshot {
    /// Works out everything a snapshot stores beyond the parsed save: derived player
    /// columns and the metadata summary. `source` carries the snapshot id, the save file
    /// fields and the import time; the rest of the metadata is filled in here.
    pub fn from_parsed(save: ParsedSave, source: SnapshotMeta) -> Self {
        let ids: BTreeSet<u32> = save
            .players
            .iter()
            .flat_map(|p| {
                std::iter::once(p.nation_id as u32).chain(p.other_nation_ids.iter().copied())
            })
            .collect();
        let mut nations: Vec<_> = ids
            .into_iter()
            .map(|id| NamedId {
                id,
                name: nation_name(id),
            })
            .collect();
        nations.sort_by_key(|v| normalize(&v.name));
        let used: BTreeSet<_> = save.players.iter().filter_map(|p| p.club_id).collect();
        let mut clubs: Vec<_> = save
            .clubs
            .iter()
            .filter(|c| used.contains(&c.id))
            .map(|c| NamedId {
                id: c.id,
                name: c.short_name.clone(),
            })
            .collect();
        clubs.sort_by_key(|v| normalize(&v.name));
        let meta = SnapshotMeta {
            name: save.name,
            game_date: save.game_date,
            format_version: save.format_version,
            player_count: save.players.len(),
            diagnostics: save.diagnostics,
            warnings: save.warnings,
            nations,
            clubs,
            managed_club_id: save.managed_club_id,
            ..source
        };
        let players = save
            .players
            .into_iter()
            .map(|player| {
                let mut nation_ids = vec![player.nation_id as u32];
                nation_ids.extend(&player.other_nation_ids);
                let nationalities: Vec<_> = nation_ids.iter().map(|&id| nation_name(id)).collect();
                PlayerRecord {
                    id: player.id,
                    uid: player.uid,
                    name: player.name.clone(),
                    full_name: player.full_name.clone(),
                    search_name: normalize(&format!(
                        "{} {} {}",
                        player.name, player.full_name, player.uid
                    )),
                    age: player.age,
                    club_id: player.club_id,
                    club: player.club.clone(),
                    nation_id: player.nation_id,
                    nation_ids,
                    nationalities: nationalities.clone(),
                    positions: player.positions.clone(),
                    position_mask: player.position_mask(),
                    main_position: player.main_position(),
                    ca: player.ca,
                    pa: player.pa,
                    wage: player.wage,
                    contract_end: player.contract_end.clone(),
                    // a value of 0 means the save has none, so keep it unknown
                    value: player.value.filter(|v| *v > 0),
                    attributes: player.attributes.clone(),
                    detail: PlayerDetail {
                        player,
                        nationalities,
                    },
                }
            })
            .collect();
        Self { meta, players }
    }
}
