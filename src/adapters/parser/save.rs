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
//! teams     team heads and rosters, each team's club
//! players   club, attributes, contract and value for each pair
//! fallback  club by contract votes for teams no record places
//! checks    managed club, no players, duplicate ids, file changed while reading
//! ```
use super::{
    abilities::{self, Ability},
    archive::{check, Archive},
    bytes, clubs, contract, names,
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
    let db = archive.member("game_db.dat", cancel)?;
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
    let club_map: HashMap<_, _> = clubs.iter().map(|c| (c.id, c)).collect();
    let squads = squads(&teams);
    let team_map: HashMap<_, _> = teams.iter().map(|t| (t.ordinal + 1, t)).collect();
    let mut players = vec![];
    for &(person, ability) in &linked.pairs {
        check(cancel)?;
        let club_id = team_map
            .get(&ability.team)
            .and_then(|t| t.club)
            .or_else(|| squads.get(&(person.id as u32)).copied());
        players.push(to_player(
            &db, person, ability, club_id, &club_map, year, &game_date,
        )?);
    }
    assign_clubs_by_contract(&mut players, &club_map);
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
        players: players.into_iter().map(|p| p.player).collect(),
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

/// Person id to club, from the rosters of teams that have a club. A first team (kind 100)
/// wins over reserve and youth teams.
fn squads(teams: &[Team]) -> HashMap<u32, u32> {
    let mut squads = HashMap::new();
    for team in teams {
        let Some(club) = team.club else { continue };
        for &id in &team.members {
            if team.kind == 100 || !squads.contains_key(&id) {
                squads.insert(id, club);
            }
        }
    }
    squads
}

/// A player with what the club fallback needs: its ability team and the team its contract names.
struct Parsed {
    player: Player,
    team: u32,
    contract_team: Option<u32>,
}

fn to_player(
    db: &[u8],
    person: &Person,
    ability: &Ability,
    club_id: Option<u32>,
    club_map: &HashMap<u32, &Club>,
    year: u16,
    game_date: &str,
) -> Result<Parsed> {
    let mut raw = ability.raw.clone();
    raw.extend_from_slice(&person.personality);
    let window = &db[person.offset.saturating_sub(CONTRACT_WINDOW)..person.offset];
    let contract = contract::find(window, person.id as u32);
    let birth_date = date_of(person.birth_year, person.birth_day)?;
    let player = Player {
        id: person.id as u32,
        uid: person.uid,
        name: person.name.clone(),
        full_name: person.full_name.clone(),
        age: age(year, game_date, person.birth_year, &birth_date),
        birth_date,
        nation_id: person.nation,
        other_nation_ids: person.other_nations.clone(),
        club_id,
        club: club_name(club_map, club_id),
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
    };
    Ok(Parsed {
        player,
        team: ability.team,
        contract_team: contract.team,
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

fn club_name(club_map: &HashMap<u32, &Club>, id: Option<u32>) -> Option<String> {
    id.and_then(|id| club_map.get(&id).map(|c| c.short_name.clone()))
}

/// Reserve/youth teams whose record names no club: take the club their players are contracted to.
fn assign_clubs_by_contract(players: &mut [Parsed], club_map: &HashMap<u32, &Club>) {
    let rows: Vec<_> = players
        .iter()
        .filter(|p| p.player.club_id.is_none())
        .filter_map(|p| Some((p.team, p.contract_team?)))
        .collect();
    let team_club = teams::club_by_contract(&rows, &club_map.keys().copied().collect());
    for p in players.iter_mut() {
        if let (None, Some(&id)) = (p.player.club_id, team_club.get(&p.team)) {
            p.player.club_id = Some(id);
            p.player.club = club_name(club_map, Some(id));
        }
    }
}

/// The club the user manages, named in the save summary. Only clubs with players count:
/// league names in the summary can collide with club-less records.
fn managed_club(
    archive: &mut Archive,
    clubs: &[Club],
    players: &[Parsed],
    cancel: &CancelFlag,
) -> Option<u32> {
    let with_players: HashSet<_> = players.iter().filter_map(|p| p.player.club_id).collect();
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

fn validate(players: &[Parsed], archive: &Archive) -> Result<()> {
    if players.is_empty() {
        return Err(Error::new(
            "NO_PLAYERS",
            "No supported player records were found in this save.",
        ));
    }
    let ids: HashSet<_> = players.iter().map(|p| p.player.id).collect();
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
    players: &[Parsed],
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
        "clubLinks": players.iter().filter(|p| p.player.club.is_some()).count(),
        "databaseBytes": db.len(),
        "elapsedMs": started.elapsed().as_millis(),
    })
}
