use crate::{
    domain::{
        countries::nation_name,
        entities::{NewSnapshot, ParsedSave, PlayerDetail, PlayerRecord},
    },
    utils::text::normalize,
    Error, Result,
};
use chrono::Utc;
use serde_json::{json, Value};
use std::collections::BTreeSet;

/// Works out everything a snapshot stores beyond the parsed save: derived
/// player columns and the metadata summary. `source` describes the save file.
pub fn build_snapshot(save: ParsedSave, mut source: Value) -> Result<NewSnapshot> {
    let ids: BTreeSet<u32> = save
        .players
        .iter()
        .flat_map(|p| std::iter::once(p.nation_id as u32).chain(p.other_nation_ids.iter().copied()))
        .collect();
    let mut nations: Vec<_> = ids
        .into_iter()
        .map(|id| json!({"id":id,"name":nation_name(id)}))
        .collect();
    nations.sort_by_key(|v| normalize(v["name"].as_str().unwrap_or("")));
    let used: BTreeSet<_> = save.players.iter().filter_map(|p| p.club_id).collect();
    let mut clubs: Vec<_> = save
        .clubs
        .iter()
        .filter(|c| used.contains(&c.id))
        .map(|c| json!({"id":c.id,"name":c.short_name}))
        .collect();
    clubs.sort_by_key(|v| normalize(v["name"].as_str().unwrap_or("")));
    let fields = json!({
        "name": save.name,
        "gameDate": save.game_date,
        "formatVersion": save.format_version,
        "playerCount": save.players.len(),
        "diagnostics": save.diagnostics,
        "warnings": save.warnings,
        "importedAt": Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        "nations": nations,
        "clubs": clubs,
        "managedClubId": save.managed_club_id,
    });
    source
        .as_object_mut()
        .ok_or_else(|| Error::query("Invalid snapshot metadata."))?
        .extend(fields.as_object().unwrap().clone());
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
    Ok(NewSnapshot {
        meta: source,
        players,
    })
}
