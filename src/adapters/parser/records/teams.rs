//! Team records, before the name pools: a head, then a roster somewhere after it.
//!
//! ```text
//! head    [01][kind][FF][flag]                       optional "typed" prefix
//!         [owner u32][00 x10][ordinal u32][uid u32][uid u32][10]
//!         Reserve/youth heads carry their name twice instead of the ten zero bytes.
//! roster  [FF x4][count u16][count ids u32][captain u32][vice u32]
//! ```
//! A team belongs to a club through its owner id, its name, or (in `save`) the contracts of
//! its players.
use super::{
    super::bytes::{u16, u32},
    scan::scan,
};
use crate::{
    domain::entities::Club,
    utils::cancel::{check, CancelFlag},
    Result,
};
use std::collections::{HashMap, HashSet};

pub struct Team {
    pub club: Option<u32>,
    pub ordinal: u32,
    pub kind: u8,
    pub members: Vec<u32>,
}

struct Head {
    offset: usize,
    owner: u32,
    ordinal: u32,
    uid: u32,
    typed: bool,
    kind: u8,
    flag: u8,
    /// Reserve/youth records carry their own name in place of the ten zero bytes.
    name: Option<String>,
}

/// From a head to its `10` marker, where the next head can start.
const HEAD_SIZE: usize = 26;
/// Where the roster search starts, from the head.
const ROSTER_FROM: usize = 30;
/// From a roster's `FF` run to its first id.
const ROSTER_IDS: usize = 6;
/// The furthest a roster can sit from its head.
const ROSTER_RANGE: usize = 10000;

/// Every team before `end` (the start of the name pools), with its club and members.
/// `valid` holds the person ids that exist, to tell a real roster from noise.
pub fn find(
    db: &[u8],
    clubs: &[Club],
    end: usize,
    valid: &HashSet<u32>,
    cancel: &CancelFlag,
) -> Result<Vec<Team>> {
    let club_map: HashMap<_, _> = clubs.iter().map(|c| (c.id, c)).collect();
    let heads = scan(4, end.saturating_sub(70), cancel, |offset| {
        let head = named_head(db, offset).or_else(|| plain_head(db, offset, &club_map))?;
        let next = head.offset + HEAD_SIZE;
        Some((head, next))
    })?;
    let mut teams = vec![];
    for (i, head) in heads.iter().enumerate() {
        check(cancel)?;
        let limit = heads
            .get(i + 1)
            .map_or(end, |h| h.offset)
            .min(head.offset + ROSTER_RANGE);
        teams.push(Team {
            club: club_of(head, clubs, &club_map),
            ordinal: head.ordinal,
            kind: head.kind,
            members: roster(db, head.offset + ROSTER_FROM, limit, valid),
        });
    }
    Ok(teams)
}

/// Ordinal, uid and repeated uid of the head at `offset`, if its marker and ids are plausible.
fn head_ids(db: &[u8], offset: usize) -> Option<(u32, u32, u32)> {
    let (ordinal, uid, again) = (
        u32(db, offset + 14),
        u32(db, offset + 18),
        u32(db, offset + 22),
    );
    (db[offset + HEAD_SIZE] == 10 && ordinal <= 500000 && uid != 0).then_some((ordinal, uid, again))
}

/// Reserve/youth head whose owner id sits just before `at`: the name twice (length-prefixed,
/// NUL-terminated), then ordinal, uid, uid, 10.
fn named_head(db: &[u8], at: usize) -> Option<Head> {
    let name_len = u32(db, at) as usize;
    if !(3..=40).contains(&name_len) || at < 4 || at + 2 * name_len + 40 > db.len() {
        return None;
    }
    let name = &db[at + 4..at + 4 + name_len];
    let again = at + 5 + name_len;
    if db[at + 4 + name_len] != 0
        || !name.iter().all(|c| (32..127).contains(c))
        || u32(db, again) as usize != name_len
        || db[again + 4..again + 4 + name_len] != *name
        || db[again + 4 + name_len] != 0
    {
        return None;
    }
    // The ordinal follows the second name; place the head as if it had the ten zero bytes.
    let offset = again + 5 + name_len - 14;
    let (ordinal, uid, uid_again) = head_ids(db, offset)?;
    if uid != uid_again {
        return None;
    }
    Some(Head {
        offset,
        owner: u32(db, at - 4),
        ordinal,
        uid,
        typed: false,
        kind: 100,
        flag: 0,
        name: Some(String::from_utf8_lossy(name).into_owned()),
    })
}

/// Head with ten zero bytes. Kept when it has the typed prefix or its owner is a club with
/// the same uid.
fn plain_head(db: &[u8], offset: usize, club_map: &HashMap<u32, &Club>) -> Option<Head> {
    if db[offset + 4..offset + 14] != [0; 10] {
        return None;
    }
    let (ordinal, uid, again) = head_ids(db, offset)?;
    let owner = u32(db, offset);
    if owner > 100000 {
        return None;
    }
    let typed = db[offset - 4] == 1
        && db[offset - 2] == 255
        && [18, 19, 20, 21, 23, 100].contains(&db[offset - 3]);
    let matched = club_map.get(&owner).is_some_and(|c| c.uid == uid) && again == uid;
    if !typed && !matched {
        return None;
    }
    Some(Head {
        offset,
        owner,
        ordinal,
        uid,
        typed,
        kind: if typed { db[offset - 3] } else { 100 },
        flag: if typed { db[offset - 1] } else { 0 },
        name: None,
    })
}

/// The club a team head belongs to, if its record says so.
fn club_of(head: &Head, clubs: &[Club], club_map: &HashMap<u32, &Club>) -> Option<u32> {
    let known = |id: &u32| club_map.contains_key(id);
    if let Some(name) = &head.name {
        // owner - 1 is the parent club; otherwise the name minus its last word ("OGC Nice 2")
        let parent = name.rsplit_once(' ').map_or(name.as_str(), |x| x.0);
        let by_name = |n: &str| {
            let mut ids = clubs.iter().filter(|c| c.short_name == n).map(|c| c.id);
            ids.next().filter(|_| ids.next().is_none())
        };
        Some(head.owner.wrapping_sub(1))
            .filter(known)
            .or_else(|| by_name(name))
            .or_else(|| by_name(parent))
    } else if club_map.get(&head.owner).is_some_and(|c| c.uid == head.uid) {
        Some(head.owner)
    } else if head.typed && head.owner <= 256 && [20, 23].contains(&head.flag) {
        // youth team of a big club: small owner ids collide with clubs, so only the flag says owner - 1
        Some(head.owner.wrapping_sub(1)).filter(known)
    } else if head.typed && !(head.kind == 100 && head.flag & 32 != 0) && head.owner > 256 {
        let id = if [20, 23].contains(&head.kind) {
            head.owner - 1
        } else {
            head.owner
        };
        Some(id).filter(known)
    } else {
        None
    }
}

/// The first roster in `from..limit` whose ids are distinct, mostly real people, and whose
/// captain and vice-captain are members.
fn roster(db: &[u8], from: usize, limit: usize, valid: &HashSet<u32>) -> Vec<u32> {
    for offset in from..limit.saturating_sub(14) {
        if db[offset] != 255 || u32(db, offset) != u32::MAX {
            continue;
        }
        let count = u16(db, offset + 4) as usize;
        let after = offset + ROSTER_IDS + count * 4;
        if !(1..=150).contains(&count) || after + 8 > limit {
            continue;
        }
        let ids: Vec<_> = (0..count)
            .map(|k| u32(db, offset + ROSTER_IDS + k * 4))
            .collect();
        let set: HashSet<_> = ids.iter().copied().collect();
        if set.len() != count || ids.iter().any(|&id| id > 3_000_000) {
            continue;
        }
        let (captain, vice) = (u32(db, after), u32(db, after + 4));
        if (captain != u32::MAX && !set.contains(&captain))
            || (vice != u32::MAX && !set.contains(&vice))
        {
            continue;
        }
        if (ids.iter().filter(|id| valid.contains(id)).count() as f64)
            < (count as f64 * 0.7).max(1.0)
        {
            continue;
        }
        return ids;
    }
    vec![]
}

/// Each player's club, by the first rule that gives one:
/// 0. the club whose record lists their ability team (`listed`, from `clubs::team_lists`),
/// 1. the team record their ability block names,
/// 2. a roster they are on (a first team wins over reserve and youth teams),
/// 3. the club most contracts of their ability team name.
///
/// `players` are (person id, ability team, contract team); `clubs` holds the club ids.
pub fn player_clubs(
    teams: &[Team],
    players: &[(u32, u32, Option<u32>)],
    clubs: &HashSet<u32>,
    listed: &HashMap<u32, u32>,
) -> Vec<Option<u32>> {
    let by_ordinal: HashMap<_, _> = teams.iter().map(|t| (t.ordinal + 1, t)).collect();
    let squads = squads(teams);
    let mut found: Vec<_> = players
        .iter()
        .map(|&(person, team, _)| {
            // free agents (team u32::MAX) can still sit on a stale roster
            if team == u32::MAX {
                return None;
            }
            listed
                .get(&team)
                .copied()
                .or_else(|| by_ordinal.get(&team).and_then(|t| t.club))
                .or_else(|| squads.get(&person).copied())
        })
        .collect();
    let rows: Vec<_> = players
        .iter()
        .zip(&found)
        .filter(|(_, club)| club.is_none())
        // free agents (team u32::MAX) keep stale contract fields; they must stay clubless
        .filter(|((_, team, _), _)| *team != u32::MAX)
        // the contract's team field is a team ordinal + 1, like `team`; vote with that team's club
        .filter_map(|(&(_, team, contract), _)| Some((team, by_ordinal.get(&contract?)?.club?)))
        .collect();
    let votes = club_by_contract(&rows, clubs);
    for (club, &(_, team, _)) in found.iter_mut().zip(players) {
        if club.is_none() {
            *club = votes.get(&team).copied();
        }
    }
    found
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

/// For teams whose own record names no club: the club most of their players' contracts name.
/// `rows` are (team id, club named by a player's contract). A tie, or a field that is not a club, gives nothing.
pub fn club_by_contract(rows: &[(u32, u32)], clubs: &HashSet<u32>) -> HashMap<u32, u32> {
    let mut votes: HashMap<u32, HashMap<u32, u32>> = HashMap::new();
    for &(team, club) in rows.iter().filter(|r| clubs.contains(&r.1)) {
        *votes.entry(team).or_default().entry(club).or_default() += 1;
    }
    votes
        .into_iter()
        .filter_map(|(team, v)| {
            let top = v.values().max().copied()?;
            let mut best = v.iter().filter(|x| *x.1 == top);
            let (club, _) = best.next()?;
            best.next().is_none().then_some((team, *club))
        })
        .collect()
}
