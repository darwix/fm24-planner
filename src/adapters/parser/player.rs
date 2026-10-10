//! Assembles the domain `Player` from records already linked: person, ability block,
//! contract and club. No byte scanning happens here.
use super::{
    meta::date_of,
    records::{
        abilities::{Ability, Linked},
        contract::Contract,
        people::Person,
        teams::{self, Team},
    },
};
use crate::{
    domain::entities::{Club, Player, CATALOG},
    Result,
};
use serde_json::json;
use std::collections::{BTreeMap, HashMap};

/// One `Player` per linked pair, `contracts` in the same order as the pairs.
pub fn build_all(
    linked: &Linked,
    contracts: Vec<Contract>,
    teams: &[Team],
    clubs: &[Club],
    year: u16,
    game_date: &str,
) -> Result<Vec<Player>> {
    let rows: Vec<_> = linked
        .pairs
        .iter()
        .zip(&contracts)
        .map(|(&(person, ability), contract)| (person.id as u32, ability.team, contract.team))
        .collect();
    let club_map: HashMap<_, _> = clubs.iter().map(|c| (c.id, c)).collect();
    let club_ids = teams::player_clubs(teams, &rows, &club_map.keys().copied().collect());
    linked
        .pairs
        .iter()
        .zip(contracts)
        .zip(club_ids)
        .map(|((&(person, ability), contract), club_id)| {
            let club = club_id.and_then(|id| club_map.get(&id).map(|c| c.short_name.clone()));
            let age = |birth_date: &str| age(year, game_date, person.birth_year, birth_date);
            let contract = contract.expire_stale(ability.team, game_date);
            build(person, ability, contract, club_id, club, age)
        })
        .collect()
}

fn build(
    person: &Person,
    ability: &Ability,
    contract: Contract,
    club_id: Option<u32>,
    club: Option<String>,
    age: impl Fn(&str) -> i32,
) -> Result<Player> {
    let mut raw = ability.raw.clone();
    raw.extend_from_slice(&person.personality);
    let birth_date = date_of(person.birth_year, person.birth_day)?;
    Ok(Player {
        id: person.id as u32,
        uid: person.uid,
        name: person.name.clone(),
        full_name: person.full_name.clone(),
        age: age(&birth_date),
        birth_date,
        nation_id: person.nation,
        other_nation_ids: person.other_nations.clone(),
        club_id,
        club,
        ca: ability.ca,
        pa: ability.pa,
        positions: positions(&ability.positions),
        position_ratings: ability.positions.clone(),
        attributes: attributes(&raw),
        raw_attributes: raw,
        wage: contract.wage,
        contract_end: contract.contract_end,
        value: contract.value,
        source: json!({
            "person": person.offset,
            "ability": ability.offset,
            "identity": person.identity_offset,
        }),
    })
}

/// Age on the game date: years between, minus one if the birthday has not come yet.
fn age(year: u16, game_date: &str, birth_year: u16, birth_date: &str) -> i32 {
    year as i32 - birth_year as i32 - i32::from(game_date[5..] < birth_date[5..])
}

/// Positions rated 15 or more (out of 20).
fn positions(ratings: &[u8]) -> Vec<String> {
    CATALOG
        .positions
        .iter()
        .zip(ratings)
        .filter(|(_, &rating)| rating >= 15)
        .map(|(position, _)| position.clone())
        .collect()
}

/// Attribute values by key. Ratings stored out of 100 (`scale` 5) become 1-20.
fn attributes(raw: &[u8]) -> BTreeMap<String, u8> {
    CATALOG
        .attributes
        .iter()
        .map(|a| {
            let value = if a.scale == 5 {
                ((raw[a.index] as u16 + 2) / 5).clamp(1, 20) as u8
            } else {
                raw[a.index]
            };
            (a.key.clone(), value)
        })
        .collect()
}
