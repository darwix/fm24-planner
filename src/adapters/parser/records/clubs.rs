//! Club records, before the name pools.
//!
//! ```text
//! [id u32][uid u32][uid u32][00][nation u32][FF x4] .. [nation u32] .. [name len u32]   43 bytes
//! [name][short name len u32][short name]
//! ..   [FF x4 or a club id][00 00 00][count][count team numbers]   the club's teams, first team first
//! ```
use super::{
    super::bytes::u32,
    scan::{is_set, scan, text},
};
use crate::{domain::entities::Club, utils::cancel::CancelFlag, Result};
use std::collections::{HashMap, HashSet};

struct Head {
    id: u32,
    uid: u32,
    nation: u32,
    name_len: usize,
}
impl Head {
    const SIZE: usize = 43;
    /// Bytes the scan needs before it trusts a match.
    const MIN_ROOM: usize = 50;
    /// None unless the separators match and the uid and nation repeat.
    fn read(db: &[u8], offset: usize) -> Option<Self> {
        if db[offset + 12] != 0 || db[offset + 17..offset + 21] != [255; 4] {
            return None;
        }
        let (uid, nation) = (u32(db, offset + 4), u32(db, offset + 13));
        (u32(db, offset + 25) == nation && u32(db, offset + 8) == uid).then(|| Self {
            id: u32(db, offset),
            uid,
            nation,
            name_len: u32(db, offset + 39) as usize,
        })
    }
}

/// Every club record before `end` (the start of the name pools).
pub fn find(db: &[u8], end: usize, cancel: &CancelFlag) -> Result<Vec<Club>> {
    scan(0, end.saturating_sub(Head::MIN_ROOM), cancel, |offset| {
        read_club(db, offset, end)
    })
}

/// The club at `offset` and where it ends.
fn read_club(db: &[u8], offset: usize, end: usize) -> Option<(Club, usize)> {
    let head = Head::read(db, offset)?;
    if head.nation > 255 || !is_set(head.uid) || head.id > 100000 {
        return None;
    }
    let name_len = head.name_len;
    let name_at = offset + Head::SIZE;
    let short_at = name_at + name_len + 4;
    if !(2..=200).contains(&name_len) || short_at >= end {
        return None;
    }
    let short_len = u32(db, name_at + name_len) as usize;
    if !(1..=100).contains(&short_len) || short_at + short_len > end {
        return None;
    }
    let club = Club {
        id: head.id,
        uid: head.uid,
        nation: head.nation,
        name: text(&db[name_at..name_at + name_len])?,
        short_name: text(&db[short_at..short_at + short_len])?,
        offset,
    };
    Some((club, short_at + short_len))
}

/// How far after its short name a club's team list can sit. Short of the next club record, it
/// can lie past a record the scan misses (A. Getxo's sits 18 KB on).
const LIST_RANGE: usize = 40000;

/// Team number (as players' ability blocks hold it) to the club whose record lists it. Teams
/// that more than one club lists are left out.
pub fn team_lists(db: &[u8], clubs: &[Club]) -> HashMap<u32, u32> {
    // id 0 is a club too, but zeros before [00 00 00 n] are padding, not a marker
    let ids: HashSet<u32> = clubs.iter().map(|c| c.id).filter(|&id| id != 0).collect();
    let mut by_offset: Vec<_> = clubs.iter().collect();
    by_offset.sort_by_key(|c| c.offset);
    let mut owners: HashMap<u32, Vec<u32>> = HashMap::new();
    for (i, club) in by_offset.iter().enumerate() {
        let name_end = club.offset + Head::SIZE + u32(db, club.offset + 39) as usize;
        let short_end = name_end + 4 + u32(db, name_end) as usize;
        let limit = by_offset
            .get(i + 1)
            .map_or(db.len(), |c| c.offset)
            .min(short_end + LIST_RANGE)
            .min(db.len().saturating_sub(16));
        if let Some(teams) = (short_end + 8..limit).find_map(|at| team_list(db, at, &ids)) {
            for team in teams {
                owners.entry(team).or_default().push(club.id);
            }
        }
    }
    owners
        .into_iter()
        .filter(|(_, clubs)| clubs.len() == 1)
        .map(|(team, clubs)| (team, clubs[0]))
        .collect()
}

/// The team list whose first number is at `at`, if the marker before it fits.
fn team_list(db: &[u8], at: usize, ids: &HashSet<u32>) -> Option<Vec<u32>> {
    let count = db[at - 1] as usize;
    let before = u32(db, at - 8);
    if db[at - 4..at - 1] != [0; 3]
        || !(1..=4).contains(&count)
        || !(before == u32::MAX || ids.contains(&before))
    {
        return None;
    }
    let teams: Vec<u32> = (0..count).map(|k| u32(db, at + 4 * k)).collect();
    let distinct = teams.iter().collect::<HashSet<_>>().len() == count;
    (distinct && teams.iter().all(|t| (1..=200_000).contains(t))).then_some(teams)
}
