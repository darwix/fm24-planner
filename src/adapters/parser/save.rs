//! Turns a `.fm` save into players. Orchestration only; each step lives in its own module.
//!
//! ```text
//! meta      open the archive, check the version, take game_db.dat (one big byte blob)
//! meta      in-game date from the database header
//! records   names      first / last / common name pools
//!           people     person records, their ids and other nationalities
//!           abilities  CA, PA, team, positions, attributes; link each to its person
//!           clubs      club records
//!           teams      team heads and rosters
//!           contract   wage, contract end, value
//! player    each player's club (team record, roster, contract votes), then the domain Player
//! meta      managed club from the save summary
//! report    reject empty or ambiguous results, summarise what was found
//! ```
use super::{
    meta, player,
    records::{abilities, clubs, contract, names, people, teams},
    report,
};
use crate::{
    domain::entities::ParsedSave,
    utils::cancel::{check, CancelFlag},
    Result,
};
use std::{path::Path, time::Instant};

pub const PARSER_VERSION: &str = "fm24-rust-47-fm";

pub fn parse_save(
    path: &Path,
    cancel: &CancelFlag,
    progress: impl Fn(u8, &str),
) -> Result<ParsedSave> {
    let started = Instant::now();
    check(cancel)?;
    progress(2, "Reading save index");
    let (mut archive, version) = meta::open_supported(path, cancel)?;
    progress(10, "Reading player database");
    let db = archive.member("game_db.dat", cancel)?;
    let (year, game_date) = meta::game_date(&db)?;
    progress(23, "Resolving player names");
    let pools = names::find(&db, cancel)?;
    progress(35, "Reading player records");
    let people = people::find(&db, &pools, cancel)?;
    progress(52, "Reading ability and attributes");
    let abilities = abilities::find(&db, pools.end, cancel)?;
    let linked = abilities::link(&people, &abilities);
    progress(68, "Resolving clubs and squads");
    let clubs = clubs::find(&db, pools.start, cancel)?;
    let teams = teams::find(&db, &clubs, pools.start, &people::ids(&people), cancel)?;
    let listed = clubs::team_lists(&db, &clubs, &teams);
    let team_club = teams::team_clubs(&teams, &listed);
    let contracts = contract::for_people(&db, &linked, &team_club, cancel)?;
    let players = player::build_all(
        &linked, contracts, &teams, &clubs, &listed, year, &game_date,
    )?;
    let managed_club_id = meta::managed_club(&mut archive, &clubs, &players, cancel);
    report::validate(&players, &archive)?;
    progress(88, "Preparing search index");
    let diagnostics = report::diagnostics(report::Counts {
        db: &db,
        people: &people,
        abilities: &abilities,
        linked: &linked,
        clubs: &clubs,
        teams: &teams,
        players: &players,
        started,
    });
    Ok(ParsedSave {
        name: archive.name,
        format_version: version,
        game_date,
        warnings: linked.warnings(),
        players,
        clubs,
        managed_club_id,
        diagnostics,
    })
}
