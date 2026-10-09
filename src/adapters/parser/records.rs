use super::archive::check;
use crate::utils::cancel::CancelFlag;
use crate::{Error, Result};
use std::collections::{HashMap, HashSet};

pub use super::bytes::{u16, u32};
fn text(b: &[u8]) -> Option<String> {
    let s = std::str::from_utf8(b).ok()?;
    if s.chars().any(|c| c <= '\u{1f}' || c == '\u{fffd}') {
        None
    } else {
        Some(s.to_string())
    }
}
fn checkpoint(p: usize, c: &CancelFlag) -> Result<()> {
    if p & 0xffff == 0 {
        check(c)?;
    }
    Ok(())
}
/// 0 and all-ones mark an unset id.
fn is_set(v: u32) -> bool {
    v != 0 && v != u32::MAX
}

/// A name pool is `[count u32]` then, per name, `[id u32][len u32][text]`.
const POOL_COUNT: usize = 4;
const POOL_ENTRY: usize = 8;

pub struct Names {
    pub first: Vec<String>,
    pub last: Vec<String>,
    pub common: Vec<String>,
    pub start: usize,
    pub end: usize,
}
fn pool(b: &[u8], start: usize, cancel: &CancelFlag) -> Result<Option<(Vec<String>, usize)>> {
    if start + POOL_COUNT + POOL_ENTRY + 4 >= b.len() {
        return Ok(None);
    }
    let count = u32(b, start) as usize;
    if !(1..=3_000_000).contains(&count) {
        return Ok(None);
    }
    let mut names = Vec::new();
    let mut p = start + POOL_COUNT;
    for id in 0..count {
        if id % 4096 == 0 {
            check(cancel)?;
        }
        if p + POOL_ENTRY > b.len() || u32(b, p) as usize != id {
            return Ok(None);
        }
        let n = u32(b, p + 4) as usize;
        let name = p + POOL_ENTRY;
        if n > 512 || name + n > b.len() {
            return Ok(None);
        }
        let Some(s) = text(&b[name..name + n]) else {
            return Ok(None);
        };
        names.push(s);
        p = name + n;
    }
    Ok(Some((names, p)))
}
pub fn names(b: &[u8], cancel: &CancelFlag) -> Result<Names> {
    for p in 0..b.len().saturating_sub(40) {
        checkpoint(p, cancel)?;
        // A first-name pool: a large count, then entry 0 followed by entry 1.
        let entry = p + POOL_COUNT;
        if b[entry..entry + 4] != [0; 4] {
            continue;
        }
        let count = u32(b, p);
        if !(10000..=3_000_000).contains(&count) {
            continue;
        }
        let n = u32(b, entry + 4) as usize;
        let next = entry + POOL_ENTRY + n;
        if !(1..=100).contains(&n) || next + 4 > b.len() || u32(b, next) != 1 {
            continue;
        }
        let Some((first, end)) = pool(b, p, cancel)? else {
            continue;
        };
        let Some((last, end)) = pool(b, end, cancel)? else {
            continue;
        };
        if last.len() < 10000 {
            continue;
        }
        let Some((common, end)) = pool(b, end, cancel)? else {
            continue;
        };
        return Ok(Names {
            first,
            last,
            common,
            start: p,
            end,
        });
    }
    Err(Error::new(
        "UNSUPPORTED_NAMES",
        "Could not locate the FM24 name tables in this save.",
    ))
}

pub struct Person {
    pub offset: usize,
    pub end: usize,
    pub id: i64,
    pub uid: u32,
    pub name: String,
    pub full_name: String,
    pub birth_day: u16,
    pub birth_year: u16,
    pub nation: u16,
    pub personality: Vec<u8>,
    pub other_nations: Vec<u32>,
    pub identity_offset: usize,
}
struct Identity {
    offset: usize,
    id: u32,
    uid: u32,
}
/// Name `id` in `pool`, if the id is in range.
fn pick(pool: &[String], id: u32) -> Option<&str> {
    pool.get(id as usize).map(String::as_str)
}
/// The nickname (common name) wins when the player has one, else first + last.
pub fn display_name(first: &str, last: &str, common: Option<&str>) -> String {
    match common {
        Some(c) if !c.is_empty() => c.to_string(),
        _ => format!("{first} {last}").trim().to_string(),
    }
}
/// Fixed header of a person record: name ids and the full-name length, each
/// followed by a zero byte. The full name follows it, then a [`PersonTail`].
struct PersonHead {
    first: u32,
    last: u32,
    common: u32,
    name_len: usize,
}
impl PersonHead {
    const SIZE: usize = 19;
    /// None unless the separator bytes are zero.
    fn read(b: &[u8], at: usize) -> Option<Self> {
        let zero = |i: usize| b[at + i] == 0;
        (zero(4) && zero(9) && zero(14) && zero(17) && zero(18)).then(|| Self {
            first: u32(b, at),
            last: u32(b, at + 5),
            common: u32(b, at + 10),
            name_len: u32(b, at + 15) as usize,
        })
    }
}

/// Fields right after the full name: birth date, nation, personality, then
/// the list of other nationalities.
struct PersonTail {
    birth_day: u16,
    birth_year: u16,
    nation: u16,
    personality: [u8; 8],
}
impl PersonTail {
    const SIZE: usize = 25;
    /// Bytes the scan needs past the name before it trusts a match.
    const MIN_ROOM: usize = 45;
    /// `1` when an other-nationality list follows.
    const NATIONS_FLAG: usize = 33;
    const NATIONS_COUNT: usize = 34;
    const NATIONS: usize = 35;
    /// None unless the six bytes before the personality are zero.
    fn read(b: &[u8], at: usize) -> Option<Self> {
        (b[at + 11..at + 17] == [0; 6]).then(|| Self {
            birth_day: u16(b, at),
            birth_year: u16(b, at + 2),
            nation: u16(b, at + 9),
            personality: b[at + 17..at + 25].try_into().unwrap(),
        })
    }
    fn plausible(&self) -> bool {
        (1..=366).contains(&self.birth_day)
            && (1850..=2300).contains(&self.birth_year)
            && self.nation <= 255
            && self.personality.iter().all(|v| (1..=20).contains(v))
    }
}

/// One 16-byte entry of the other-nationality list: `[nation u32][00 x4] .. [08][09 or 70]`.
const OTHER_NATION_SIZE: usize = 16;
fn other_nation(b: &[u8], at: usize) -> Option<u32> {
    let nation = u32(b, at);
    (nation < 256
        && b[at + 4..at + 8] == [0; 4]
        && b[at + 10] == 8
        && [9, 70].contains(&b[at + 11]))
    .then_some(nation)
}

/// Bytes from an identity's id that [`identity`] reads.
const IDENTITY_SIZE: usize = 12;
/// Entity identity at `p`: `[id u32][uid u32][uid again, or a source id u32]`, preceded by a
/// type byte and three zero bytes. When the uid is not repeated, a birth date and flags sit
/// just before it.
fn identity(b: &[u8], p: usize, max_id: u64) -> Option<Identity> {
    if b[p - 1] != 0
        || b[p - 2] != 0
        || b[p - 3] != 0
        || b[p - 7] & 7 > 2
        || ![0, 1, 4, 5].contains(&b[p - 4])
    {
        return None;
    }
    let (id, uid, source) = (u32(b, p), u32(b, p + 4), u32(b, p + 8));
    if id as u64 >= max_id || !is_set(uid) {
        return None;
    }
    if uid != source {
        if p < 12 {
            return None;
        }
        let (day, year, flags) = (u16(b, p - 12) & 511, u16(b, p - 10), b[p - 6]);
        if !(1..=366).contains(&day)
            || !(1900..=2300).contains(&year)
            || flags & 0x85 != 0
            || flags & 0x60 == 0
            || source < 1000
            || source == u32::MAX
        {
            return None;
        }
    }
    // Round ids whose bytes repeat are padding, not an identity.
    if id & 255 == 0 && uid & 255 == 0 && u32(b, p + 5) == u32(b, p + 9) {
        return None;
    }
    Some(Identity { offset: p, id, uid })
}

pub fn people(b: &[u8], names: &Names, cancel: &CancelFlag) -> Result<Vec<Person>> {
    let mut people = Vec::new();
    let mut p = names.end;
    while p < b.len().saturating_sub(100) {
        checkpoint(p, cancel)?;
        let start = p;
        p += 1;
        let Some(head) = PersonHead::read(b, start) else {
            continue;
        };
        if (head.first != u32::MAX && head.first as usize >= names.first.len())
            || (head.last != u32::MAX && head.last as usize >= names.last.len())
            || head.name_len > 200
        {
            continue;
        }
        let end = start + PersonHead::SIZE + head.name_len;
        if end + PersonTail::MIN_ROOM > b.len() {
            continue;
        }
        let Some(tail) = PersonTail::read(b, end).filter(PersonTail::plausible) else {
            continue;
        };
        let Some(full_name) = text(&b[start + PersonHead::SIZE..end]) else {
            continue;
        };
        let name = Some(display_name(
            pick(&names.first, head.first).unwrap_or(""),
            pick(&names.last, head.last).unwrap_or(""),
            pick(&names.common, head.common),
        ))
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| full_name.clone());
        if name.is_empty() {
            continue;
        }
        people.push(Person {
            offset: start,
            end,
            id: -1,
            uid: u32::MAX,
            name: name.clone(),
            full_name: if full_name.is_empty() {
                name
            } else {
                full_name
            },
            birth_day: tail.birth_day,
            birth_year: tail.birth_year,
            nation: tail.nation,
            personality: tail.personality.to_vec(),
            other_nations: vec![],
            identity_offset: 0,
        });
        p = end + PersonTail::SIZE;
    }
    if names.end + 4 > b.len() {
        return Err(Error::new("TRUNCATED", "The person table is incomplete."));
    }
    let max_id = u32(b, names.end) as u64 + 1;
    let mut identities = Vec::new();
    for p in names.end + 7..b.len().saturating_sub(IDENTITY_SIZE) {
        checkpoint(p, cancel)?;
        identities.extend(identity(b, p, max_id));
    }
    // Reproduce the strictly ascending entity chain, including tie replacement.
    let mut tails: Vec<u32> = vec![];
    let mut indices: Vec<usize> = vec![];
    let mut prev = vec![None; identities.len()];
    for (i, identity) in identities.iter().enumerate() {
        if i % 4096 == 0 {
            check(cancel)?;
        }
        let pos = tails.partition_point(|id| *id < identity.id);
        prev[i] = pos.checked_sub(1).map(|j| indices[j]);
        if pos == tails.len() {
            tails.push(identity.id);
            indices.push(i);
        } else {
            tails[pos] = identity.id;
            indices[pos] = i;
        }
    }
    let mut ordered = vec![];
    let mut ix = indices.last().copied();
    while let Some(i) = ix {
        ordered.push(&identities[i]);
        ix = prev[i];
    }
    ordered.reverse();
    let mut j = 0;
    for i in 0..people.len() {
        if i % 4096 == 0 {
            check(cancel)?;
        }
        let end = people.get(i + 1).map_or(b.len(), |p| p.offset);
        let person = &mut people[i];
        while j < ordered.len() && ordered[j].offset < person.end + PersonTail::SIZE {
            j += 1;
        }
        let Some(identity) = ordered.get(j).filter(|x| x.offset < end) else {
            continue;
        };
        person.id = identity.id as i64;
        person.uid = identity.uid;
        person.identity_offset = identity.offset;
        let count = b[person.end + PersonTail::NATIONS_COUNT] as usize;
        let list = person.end + PersonTail::NATIONS;
        if b[person.end + PersonTail::NATIONS_FLAG] == 1
            && list + count * OTHER_NATION_SIZE <= identity.offset
        {
            for k in 0..count {
                let Some(nation) = other_nation(b, list + k * OTHER_NATION_SIZE) else {
                    continue;
                };
                if nation != person.nation as u32 && !person.other_nations.contains(&nation) {
                    person.other_nations.push(nation);
                }
            }
        }
    }
    Ok(people)
}

pub struct Ability {
    pub offset: usize,
    pub owner_id: u64,
    pub ca: u8,
    pub pa: u8,
    pub team: u32,
    pub positions: Vec<u8>,
    pub raw: Vec<u8>,
}
/// Ability block, read from `at`, the start of its attribute ratings:
/// `[owner u32][uid u32][source u32] .. [ca][00][pa][00] .. [team u32] .. [15 position ratings][54 attributes]`.
impl Ability {
    /// Bytes from the owner id to the attributes.
    const BEFORE: usize = 57;
    pub(super) const ATTRIBUTES: usize = 54;
    fn read(b: &[u8], at: usize) -> Option<Self> {
        let (ca, pa) = (b[at - 38], b[at - 36]);
        let positions = &b[at - 15..at];
        let attributes = &b[at..at + Self::ATTRIBUTES];
        let ok = b[at - 37] == 0
            && b[at - 35] == 0
            && (1..=200).contains(&ca)
            && (1..=200).contains(&pa)
            && positions.iter().all(|v| (1..=20).contains(v))
            && positions.contains(&20)
            && attributes.iter().all(|v| (1..=100).contains(v))
            && is_set(u32(b, at - 53))
            && is_set(u32(b, at - 49));
        ok.then(|| Self {
            offset: at,
            owner_id: u32(b, at - 57) as u64 + 1,
            ca,
            pa,
            team: u32(b, at - 23),
            positions: positions.to_vec(),
            raw: attributes.to_vec(),
        })
    }
}

pub fn abilities(b: &[u8], start: usize, cancel: &CancelFlag) -> Result<Vec<Ability>> {
    let mut blocks = vec![];
    let mut p = start + Ability::BEFORE;
    while p < b.len().saturating_sub(Ability::ATTRIBUTES) {
        checkpoint(p, cancel)?;
        let at = p;
        p += 1;
        if let Some(block) = Ability::read(b, at) {
            blocks.push(block);
            p = at + Ability::ATTRIBUTES;
        }
    }
    Ok(blocks)
}

pub use crate::domain::entities::Club;

/// Fixed head of a club record: `[id u32][uid u32][uid u32][00][nation u32][FF x4] ..
/// [nation u32] .. [name len u32]`, then the name, `[short name len u32]` and the short name.
struct ClubHead {
    id: u32,
    uid: u32,
    nation: u32,
    name_len: usize,
}
impl ClubHead {
    const SIZE: usize = 43;
    /// Bytes the scan needs before it trusts a match.
    const MIN_ROOM: usize = 50;
    /// None unless the separators match and the uid and nation repeat.
    fn read(b: &[u8], at: usize) -> Option<Self> {
        if b[at + 12] != 0 || b[at + 17..at + 21] != [255; 4] {
            return None;
        }
        let (uid, nation) = (u32(b, at + 4), u32(b, at + 13));
        (u32(b, at + 25) == nation && u32(b, at + 8) == uid).then(|| Self {
            id: u32(b, at),
            uid,
            nation,
            name_len: u32(b, at + 39) as usize,
        })
    }
}

pub fn clubs(b: &[u8], end: usize, cancel: &CancelFlag) -> Result<Vec<Club>> {
    let mut clubs = vec![];
    let mut p = 0;
    while p + ClubHead::MIN_ROOM < end {
        checkpoint(p, cancel)?;
        let at = p;
        p += 1;
        let Some(head) = ClubHead::read(b, at) else {
            continue;
        };
        if head.nation > 255 || !is_set(head.uid) || head.id > 100000 {
            continue;
        }
        let n = head.name_len;
        let name_at = at + ClubHead::SIZE;
        let short_at = name_at + n + 4;
        if !(2..=200).contains(&n) || short_at >= end {
            continue;
        }
        let sn = u32(b, name_at + n) as usize;
        if !(1..=100).contains(&sn) || short_at + sn > end {
            continue;
        }
        let Some(name) = text(&b[name_at..name_at + n]) else {
            continue;
        };
        let Some(short_name) = text(&b[short_at..short_at + sn]) else {
            continue;
        };
        clubs.push(Club {
            id: head.id,
            uid: head.uid,
            nation: head.nation,
            name,
            short_name,
            offset: at,
        });
        p = short_at + sn;
    }
    Ok(clubs)
}

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
/// Team head: `[owner u32][00 x10][ordinal u32][uid u32][uid u32][10]`, optionally preceded by
/// `[01][kind][FF][flag]`. Reserve/youth heads carry their name in place of the zero bytes.
/// The next head can start at the `10` marker.
const HEAD_SIZE: usize = 26;
/// Where the roster search starts, from the head.
const ROSTER_FROM: usize = 30;
/// Roster: `[FF x4][count u16][count ids u32][captain u32][vice u32]`.
const ROSTER_IDS: usize = 6;
/// Ordinal, uid and repeated uid of the head at `offset`, if its marker and ids are plausible.
fn head_ids(b: &[u8], offset: usize) -> Option<(u32, u32, u32)> {
    let (ordinal, uid, again) = (
        u32(b, offset + 14),
        u32(b, offset + 18),
        u32(b, offset + 22),
    );
    (b[offset + HEAD_SIZE] == 10 && ordinal <= 500000 && uid != 0).then_some((ordinal, uid, again))
}
/// Reserve/youth record: owner, name twice (length-prefixed, NUL-terminated), then ordinal, uid, uid, 10.
fn named_head(b: &[u8], p: usize) -> Option<Head> {
    let l = u32(b, p) as usize;
    if !(3..=40).contains(&l) || p < 4 || p + 2 * l + 40 > b.len() {
        return None;
    }
    let text = &b[p + 4..p + 4 + l];
    let q = p + 5 + l;
    if b[p + 4 + l] != 0
        || !text.iter().all(|c| (32..127).contains(c))
        || u32(b, q) as usize != l
        || b[q + 4..q + 4 + l] != *text
        || b[q + 4 + l] != 0
    {
        return None;
    }
    // The ordinal follows the second name.
    let offset = q + 5 + l - 14;
    let (ordinal, uid, again) = head_ids(b, offset)?;
    if uid != again {
        return None;
    }
    Some(Head {
        offset,
        owner: u32(b, p - 4),
        ordinal,
        uid,
        typed: false,
        kind: 100,
        flag: 0,
        name: Some(String::from_utf8_lossy(text).into_owned()),
    })
}
/// For teams whose own record names no club: the club most of their players' contracts name.
/// `rows` are (team id, contract team field). A tie, or a field that is not a club, gives nothing.
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

pub fn teams(
    b: &[u8],
    clubs: &[Club],
    end: usize,
    valid: &HashSet<u32>,
    cancel: &CancelFlag,
) -> Result<Vec<Team>> {
    let club_map: HashMap<_, _> = clubs.iter().map(|c| (c.id, c)).collect();
    let mut heads = vec![];
    let mut p = 4;
    while p < end.saturating_sub(70) {
        checkpoint(p, cancel)?;
        let at = p;
        p += 1;
        if let Some(h) = named_head(b, at) {
            p = h.offset + HEAD_SIZE;
            heads.push(h);
            continue;
        }
        if b[at + 4..at + 14] != [0; 10] {
            continue;
        }
        let Some((ordinal, uid, again)) = head_ids(b, at) else {
            continue;
        };
        let owner = u32(b, at);
        if owner > 100000 {
            continue;
        }
        let typed =
            b[at - 4] == 1 && b[at - 2] == 255 && [18, 19, 20, 21, 23, 100].contains(&b[at - 3]);
        let matched = club_map.get(&owner).is_some_and(|c| c.uid == uid) && again == uid;
        if !typed && !matched {
            continue;
        }
        heads.push(Head {
            offset: at,
            owner,
            ordinal,
            uid,
            typed,
            kind: if typed { b[at - 3] } else { 100 },
            flag: if typed { b[at - 1] } else { 0 },
            name: None,
        });
        p = at + HEAD_SIZE;
    }
    let mut teams = vec![];
    for (i, h) in heads.iter().enumerate() {
        check(cancel)?;
        let limit = heads
            .get(i + 1)
            .map_or(end, |h| h.offset)
            .min(h.offset + 10000);
        let mut club = None;
        if let Some(name) = &h.name {
            // owner - 1 is the parent club; otherwise the name minus its last word ("OGC Nice 2")
            let parent = name.rsplit_once(' ').map_or(name.as_str(), |x| x.0);
            let by_name = |n: &str| {
                let mut ids = clubs.iter().filter(|c| c.short_name == n).map(|c| c.id);
                ids.next().filter(|_| ids.next().is_none())
            };
            club = Some(h.owner.wrapping_sub(1))
                .filter(|id| club_map.contains_key(id))
                .or_else(|| by_name(name))
                .or_else(|| by_name(parent));
        } else if club_map.get(&h.owner).is_some_and(|c| c.uid == h.uid) {
            club = Some(h.owner);
        } else if h.typed && h.owner <= 256 && [20, 23].contains(&h.flag) {
            // youth team of a big club: small owner ids collide with clubs, so only the flag says owner - 1
            club = Some(h.owner.wrapping_sub(1)).filter(|id| club_map.contains_key(id));
        } else if h.typed && !(h.kind == 100 && h.flag & 32 != 0) && h.owner > 256 {
            let id = if [20, 23].contains(&h.kind) {
                h.owner - 1
            } else {
                h.owner
            };
            if club_map.contains_key(&id) {
                club = Some(id);
            }
        }
        let mut members = vec![];
        for p in h.offset + ROSTER_FROM..limit.saturating_sub(14) {
            if b[p] != 255 || u32(b, p) != u32::MAX {
                continue;
            }
            let count = u16(b, p + 4) as usize;
            let after = p + ROSTER_IDS + count * 4;
            if !(1..=150).contains(&count) || after + 8 > limit {
                continue;
            }
            let ids: Vec<_> = (0..count).map(|k| u32(b, p + ROSTER_IDS + k * 4)).collect();
            let set: HashSet<_> = ids.iter().copied().collect();
            if set.len() != count || ids.iter().any(|&id| id > 3_000_000) {
                continue;
            }
            let (captain, vice) = (u32(b, after), u32(b, after + 4));
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
            members = ids;
            break;
        }
        teams.push(Team {
            club,
            ordinal: h.ordinal,
            kind: h.kind,
            members,
        });
    }
    Ok(teams)
}
