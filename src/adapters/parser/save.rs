//! Turns a `.fm` save into players. The pipeline:
//!
//! ```text
//! archive   open the container, check the version, take game_db.dat (one big byte blob)
//! date      in-game date from the database header
//! names     first / last / common name pools
//! people    person records, their ids and other nationalities
//! abilities CA, PA, team, positions and attributes
//! link      pair each person with the one ability block before it
//! clubs     club records
//! teams     team heads and rosters; each player's club (team record, roster, contract votes)
//! players   contract, attributes and value for each pair
//! checks    managed club, no players, duplicate ids, file changed while reading
//! ```
use super::{
    abilities::{self, Ability},
    archive::{check, Archive},
    bytes, clubs, contract,
    contract::Contract,
    names,
    people::{self, Person},
    teams::{self, Team},
};
use crate::utils::cancel::CancelFlag;
use crate::{
    domain::{
        catalog::CATALOG,
        entities::{Club, ParsedSave, Player},
    },
    Error, Result,
};
use chrono::{Duration, NaiveDate};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::Path,
    time::Instant,
};

pub const PARSER_VERSION: &str = "fm24-rust-16-fm";
const SUPPORTED_VERSION: &str = "24.3.0+0";
/// How far before a person record its contract can sit.
const CONTRACT_WINDOW: usize = 800;

pub fn parse_save(
    path: &Path,
    cancel: &CancelFlag,
    progress: impl Fn(u8, &str),
) -> Result<ParsedSave> {
    let started = Instant::now();
    check(cancel)?;
    progress(2, "Reading save index");
    let (mut archive, version) = open_supported(path, cancel)?;
    progress(10, "Reading player database");
    let db: Vec<u8> = archive.member("game_db.dat", cancel)?;
    let (year, game_date) = game_date(&db)?;
    progress(23, "Resolving player names");
    let pools = names::find(&db, cancel)?;
    progress(35, "Reading player records");
    let people = people::find(&db, &pools, cancel)?;
    progress(52, "Reading ability and attributes");
    let abilities = abilities::find(&db, pools.end, cancel)?;
    let linked = link(&people, &abilities);
    progress(68, "Resolving clubs and squads");
    let clubs = clubs::find(&db, pools.start, cancel)?;
    let teams = teams::find(&db, &clubs, pools.start, &person_ids(&people), cancel)?;
    let mut contracts = vec![];
    for &(person, _) in &linked.pairs {
        check(cancel)?;
        let window = &db[person.offset.saturating_sub(CONTRACT_WINDOW)..person.offset];
        contracts.push(contract::find(window, person.id as u32));
    }
    let rows: Vec<_> = linked
        .pairs
        .iter()
        .zip(&contracts)
        .map(|(&(person, ability), contract)| (person.id as u32, ability.team, contract.team))
        .collect();
    let club_map: HashMap<_, _> = clubs.iter().map(|c| (c.id, c)).collect();
    let club_ids = teams::player_clubs(&teams, &rows, &club_map.keys().copied().collect());
    let players = linked
        .pairs
        .iter()
        .zip(contracts)
        .zip(club_ids)
        .map(|((&(person, ability), contract), club_id)| {
            let club = club_id.and_then(|id| club_map.get(&id).map(|c| c.short_name.clone()));
            to_player(
                &db, person, ability, contract, club_id, club, year, &game_date,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    let managed_club_id = managed_club(&mut archive, &clubs, &players, cancel);
    validate(&players, &archive)?;
    progress(88, "Preparing search index");
    let diagnostics = diagnostics(
        &db, &people, &abilities, &linked, &clubs, &teams, &players, started,
    );
    Ok(ParsedSave {
        name: archive.name,
        format_version: version,
        game_date,
        warnings: warnings(&linked),
        players,
        clubs,
        managed_club_id,
        diagnostics,
    })
}

/// Opens the archive and checks it is an FM24 24.3 save.
fn open_supported(path: &Path, cancel: &CancelFlag) -> Result<(Archive, String)> {
    let mut archive = Archive::open(path, cancel)?;
    let info = archive.member("game_info.dat", cancel)?;
    let version = info
        .get(12..20)
        .and_then(|s| std::str::from_utf8(s).ok())
        .unwrap_or("unknown");
    if version != SUPPORTED_VERSION {
        return Err(Error::new(
            "UNSUPPORTED_VERSION",
            format!("Save format {version} is not supported. FM24 24.3 saves are required."),
        ));
    }
    Ok((archive, version.to_string()))
}

/// The in-game date, `[day u16][year u16]` at byte 36 of the database.
fn game_date(db: &[u8]) -> Result<(u16, String)> {
    if db.len() < 40 {
        return Err(Error::new(
            "TRUNCATED",
            "The player database header is incomplete.",
        ));
    }
    let (day, year) = (bytes::u16(db, 36) & 511, bytes::u16(db, 38));
    if !(1..=366).contains(&day) || !(2020..=2300).contains(&year) {
        return Err(Error::new(
            "UNSUPPORTED_DATE",
            "Could not read the in-game date.",
        ));
    }
    Ok((year, date_of(year, day)?))
}

pub(super) fn date_of(year: u16, day: u16) -> Result<String> {
    // Match JavaScript's one-based day-of-year rollover for day 366.
    let date = NaiveDate::from_ymd_opt(year as i32, 1, 1)
        .and_then(|d| d.checked_add_signed(Duration::days(day as i64 - 1)))
        .ok_or_else(|| Error::new("UNSUPPORTED_DATE", "Invalid in-game date."))?;
    Ok(date.format("%Y-%m-%d").to_string())
}

struct Linked<'a> {
    pairs: Vec<(&'a Person, &'a Ability)>,
    /// People with more than one ability block before them.
    ambiguous: usize,
    /// People whose one ability block belongs to someone else.
    unbound: usize,
}

/// Pairs each person with the single ability block between the previous person and them,
/// if that block names them as its owner.
fn link<'a>(people: &'a [Person], abilities: &'a [Ability]) -> Linked<'a> {
    let mut linked = Linked {
        pairs: vec![],
        ambiguous: 0,
        unbound: 0,
    };
    let mut ix = 0;
    for person in people {
        let first = ix;
        while ix < abilities.len() && abilities[ix].offset < person.offset {
            ix += 1;
        }
        match ix - first {
            0 => {}
            1 if person.id >= 0 && abilities[first].person_id == person.id as u64 => {
                linked.pairs.push((person, &abilities[first]))
            }
            1 => linked.unbound += 1,
            _ => linked.ambiguous += 1,
        }
    }
    linked
}

fn person_ids(people: &[Person]) -> HashSet<u32> {
    people
        .iter()
        .filter(|p| p.id >= 0)
        .map(|p| p.id as u32)
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn to_player(
    db: &[u8],
    person: &Person,
    ability: &Ability,
    contract: Contract,
    club_id: Option<u32>,
    club: Option<String>,
    year: u16,
    game_date: &str,
) -> Result<Player> {
    let mut raw = ability.raw.clone();
    raw.extend_from_slice(&person.personality);
    let birth_date = date_of(person.birth_year, person.birth_day)?;
    Ok(Player {
        id: person.id as u32,
        uid: person.uid,
        name: person.name.clone(),
        full_name: person.full_name.clone(),
        age: age(year, game_date, person.birth_year, &birth_date),
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
        value: contract::value(db, ability.offset),
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

/// The club the user manages, named in the save summary. Only clubs with players count:
/// league names in the summary can collide with club-less records.
fn managed_club(
    archive: &mut Archive,
    clubs: &[Club],
    players: &[Player],
    cancel: &CancelFlag,
) -> Option<u32> {
    let with_players: HashSet<_> = players.iter().filter_map(|p| p.club_id).collect();
    let names: Vec<(u32, &str)> = clubs
        .iter()
        .filter(|c| with_players.contains(&c.id))
        .flat_map(|c| [(c.id, c.name.as_str()), (c.id, c.short_name.as_str())])
        .collect();
    let summary = archive.member("save_game_summary.dat", cancel).ok()?;
    managed_club_id(&summary, &names)
}

/// Managed club: first length-prefixed string in the save summary that names exactly one club.
pub fn managed_club_id(summary: &[u8], clubs: &[(u32, &str)]) -> Option<u32> {
    (0..summary.len().saturating_sub(4)).find_map(|i| {
        let n = u32::from_le_bytes(summary[i..i + 4].try_into().ok()?) as usize;
        let s = std::str::from_utf8(summary.get(i + 4..i + 4 + n)?).ok()?;
        let mut ids = clubs.iter().filter(|c| c.1 == s).map(|c| c.0);
        let id = ids.next()?;
        ids.all(|x| x == id).then_some(id)
    })
}

fn validate(players: &[Player], archive: &Archive) -> Result<()> {
    if players.is_empty() {
        return Err(Error::new(
            "NO_PLAYERS",
            "No supported player records were found in this save.",
        ));
    }
    let ids: HashSet<_> = players.iter().map(|p| p.id).collect();
    if ids.len() != players.len() {
        return Err(Error::new(
            "AMBIGUOUS_IDENTITIES",
            "Player identities did not validate.",
        ));
    }
    if !archive.unchanged()? {
        return Err(Error::new(
            "SAVE_CHANGED",
            "The save changed during import. Wait until saving finishes, then retry.",
        ));
    }
    Ok(())
}

fn warnings(linked: &Linked) -> Vec<String> {
    let excluded = linked.ambiguous + linked.unbound;
    if excluded == 0 {
        return vec![];
    }
    vec![format!(
        "{excluded} ambiguous player records were excluded rather than assigned uncertain ratings."
    )]
}

#[allow(clippy::too_many_arguments)]
fn diagnostics(
    db: &[u8],
    people: &[Person],
    abilities: &[Ability],
    linked: &Linked,
    clubs: &[Club],
    teams: &[Team],
    players: &[Player],
    started: Instant,
) -> Value {
    json!({
        "people": people.len(),
        "identities": people.iter().filter(|p| p.id >= 0).count(),
        "abilityBlocks": abilities.len(),
        "players": players.len(),
        "ambiguous": linked.ambiguous,
        "unbound": linked.unbound,
        "clubs": clubs.len(),
        "teams": teams.len(),
        "clubLinks": players.iter().filter(|p| p.club.is_some()).count(),
        "databaseBytes": db.len(),
        "elapsedMs": started.elapsed().as_millis(),
    })
}
