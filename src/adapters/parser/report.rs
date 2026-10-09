//! End of an import: the checks that reject a bad result, and the diagnostics summary.
use super::{
    archive::Archive,
    records::{
        abilities::{Ability, Linked},
        people::Person,
        teams::Team,
    },
};
use crate::{
    domain::entities::{Club, Player},
    Error, Result,
};
use serde_json::{json, Value};
use std::{collections::HashSet, time::Instant};

/// Fails when no players were found, ids repeat, or the file changed while it was read.
pub fn validate(players: &[Player], archive: &Archive) -> Result<()> {
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

/// What each step found, stored with the snapshot.
pub struct Counts<'a> {
    pub db: &'a [u8],
    pub people: &'a [Person],
    pub abilities: &'a [Ability],
    pub linked: &'a Linked<'a>,
    pub clubs: &'a [Club],
    pub teams: &'a [Team],
    pub players: &'a [Player],
    pub started: Instant,
}

pub fn diagnostics(c: Counts) -> Value {
    json!({
        "people": c.people.len(),
        "identities": c.people.iter().filter(|p| p.id >= 0).count(),
        "abilityBlocks": c.abilities.len(),
        "players": c.players.len(),
        "ambiguous": c.linked.ambiguous,
        "unbound": c.linked.unbound,
        "clubs": c.clubs.len(),
        "teams": c.teams.len(),
        "clubLinks": c.players.iter().filter(|p| p.club.is_some()).count(),
        "databaseBytes": c.db.len(),
        "elapsedMs": c.started.elapsed().as_millis(),
    })
}
