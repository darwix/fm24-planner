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
#[derive(Clone)]
struct Identity {
    offset: usize,
    id: u32,
    uid: u32,
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

/// One entry of the other-nationality list: nation id, four zero bytes, then markers.
struct OtherNation {
    nation: u32,
}
impl OtherNation {
    const SIZE: usize = 16;
    fn read(b: &[u8], at: usize) -> Option<Self> {
        let nation = u32(b, at);
        (nation < 256
            && b[at + 4..at + 8] == [0; 4]
            && b[at + 10] == 8
            && [9, 70].contains(&b[at + 11]))
        .then_some(Self { nation })
    }
}

/// Entity identity found after a person record: id, uid, then the uid again or a source id.
/// Records whose uid is not repeated carry a date and flags just before the id.
struct IdentityRec {
    id: u32,
    uid: u32,
}
impl IdentityRec {
    /// Bytes before the id that the checks look at.
    const BACK: usize = 12;
    /// Bytes from the id that the checks look at.
    const SIZE: usize = 12;
    fn read(b: &[u8], p: usize) -> Option<Self> {
        if b[p - 1] != 0
            || b[p - 2] != 0
            || b[p - 3] != 0
            || b[p - 7] & 7 > 2
            || ![0, 1, 4, 5].contains(&b[p - 4])
        {
            return None;
        }
        Some(Self {
            id: u32(b, p),
            uid: u32(b, p + 4),
        })
    }
    /// The u32 after the uid: the uid again, or a source id.
    fn source(b: &[u8], p: usize) -> u32 {
        u32(b, p + 8)
    }
    /// Date and flags that must hold when the uid is not repeated.
    fn dated(b: &[u8], p: usize) -> bool {
        let (day, year, flags) = (u16(b, p - 12) & 511, u16(b, p - 10), b[p - 6]);
        (1..=366).contains(&day)
            && (1900..=2300).contains(&year)
            && flags & 0x85 == 0
            && flags & 0x60 != 0
    }
    /// Round ids whose bytes repeat are padding, not an identity.
    fn padding(&self, b: &[u8], p: usize) -> bool {
        self.id & 255 == 0 && self.uid & 255 == 0 && u32(b, p + 5) == u32(b, p + 9)
    }
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
        let first = names
            .first
            .get(head.first as usize)
            .map(String::as_str)
            .unwrap_or("");
        let last = names
            .last
            .get(head.last as usize)
            .map(String::as_str)
            .unwrap_or("");
        let common = names.common.get(head.common as usize).map(String::as_str);
        let name = display_name(first, last, common);
        let name = if name.is_empty() {
            full_name.clone()
        } else {
            name
        };
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
    for p in names.end + 7..b.len().saturating_sub(IdentityRec::SIZE) {
        checkpoint(p, cancel)?;
        let Some(rec) = IdentityRec::read(b, p) else {
            continue;
        };
        if rec.id as u64 >= max_id || rec.uid == 0 || rec.uid == u32::MAX {
            continue;
        }
        let source = IdentityRec::source(b, p);
        if rec.uid != source
            && (p < IdentityRec::BACK
                || !IdentityRec::dated(b, p)
                || source < 1000
                || source == u32::MAX)
        {
            continue;
        }
        if rec.padding(b, p) {
            continue;
        }
        identities.push(Identity {
            offset: p,
            id: rec.id,
            uid: rec.uid,
        });
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
        ordered.push(identities[i].clone());
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
            && list + count * OtherNation::SIZE <= identity.offset
        {
            for k in 0..count {
                let Some(other) = OtherNation::read(b, list + k * OtherNation::SIZE) else {
                    continue;
                };
                if other.nation != person.nation as u32
                    && !person.other_nations.contains(&other.nation)
                {
                    person.other_nations.push(other.nation);
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
pub(super) struct AbilityBlock<'a> {
    owner: u32,
    uid: u32,
    source: u32,
    ca: u8,
    pa: u8,
    team: u32,
    positions: &'a [u8],
    attributes: &'a [u8],
}
impl<'a> AbilityBlock<'a> {
    /// Bytes from the owner id to the attributes.
    const BEFORE: usize = 57;
    pub(super) const ATTRIBUTES: usize = 54;
    /// None unless the bytes after CA and PA are zero.
    fn read(b: &'a [u8], at: usize) -> Option<Self> {
        (b[at - 37] == 0 && b[at - 35] == 0).then(|| Self {
            owner: u32(b, at - 57),
            uid: u32(b, at - 53),
            source: u32(b, at - 49),
            ca: b[at - 38],
            pa: b[at - 36],
            team: u32(b, at - 23),
            positions: &b[at - 15..at],
            attributes: &b[at..at + Self::ATTRIBUTES],
        })
    }
    fn plausible(&self) -> bool {
        (1..=200).contains(&self.ca)
            && (1..=200).contains(&self.pa)
            && self.positions.iter().all(|v| (1..=20).contains(v))
            && self.positions.contains(&20)
            && self.attributes.iter().all(|v| (1..=100).contains(v))
            && ![0, u32::MAX].contains(&self.uid)
            && ![0, u32::MAX].contains(&self.source)
    }
}

pub fn abilities(b: &[u8], start: usize, cancel: &CancelFlag) -> Result<Vec<Ability>> {
    let mut blocks = vec![];
    let mut p = start + AbilityBlock::BEFORE;
    while p < b.len().saturating_sub(AbilityBlock::ATTRIBUTES) {
        checkpoint(p, cancel)?;
        let at = p;
        p += 1;
        let Some(block) = AbilityBlock::read(b, at).filter(AbilityBlock::plausible) else {
            continue;
        };
        blocks.push(Ability {
            offset: at,
            owner_id: block.owner as u64 + 1,
            ca: block.ca,
            pa: block.pa,
            team: block.team,
            positions: block.positions.to_vec(),
            raw: block.attributes.to_vec(),
        });
        p = at + AbilityBlock::ATTRIBUTES;
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
        if head.nation > 255 || [0, u32::MAX].contains(&head.uid) || head.id > 100000 {
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
/// `[01][kind][FF][flag]`. The roster follows somewhere after it.
impl Head {
    const ZEROS: std::ops::Range<usize> = 4..14;
    const ORDINAL: usize = 14;
    const UID: usize = 18;
    const UID_AGAIN: usize = 22;
    /// Holds 10.
    const MARKER: usize = 26;
    /// Where the next head can start.
    const SIZE: usize = 26;
    /// Where the roster search starts.
    const ROSTER_FROM: usize = 30;
}
/// Roster: `[FF x4][count u16][count ids u32][captain u32][vice u32]`.
const ROSTER_IDS: usize = 6;
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
    let offset = q + 5 + l - Head::ORDINAL;
    let (ordinal, uid) = (u32(b, offset + Head::ORDINAL), u32(b, offset + Head::UID));
    if uid == 0
        || uid != u32(b, offset + Head::UID_AGAIN)
        || b[offset + Head::MARKER] != 10
        || ordinal > 500000
    {
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
            p = h.offset + Head::SIZE;
            heads.push(h);
            continue;
        }
        if b[at + Head::MARKER] != 10 || b[at + Head::ZEROS.start..at + Head::ZEROS.end] != [0; 10]
        {
            continue;
        }
        let (owner, ordinal, uid) = (
            u32(b, at),
            u32(b, at + Head::ORDINAL),
            u32(b, at + Head::UID),
        );
        if owner > 100000 || ordinal > 500000 || uid == 0 {
            continue;
        }
        let typed =
            b[at - 4] == 1 && b[at - 2] == 255 && [18, 19, 20, 21, 23, 100].contains(&b[at - 3]);
        let matched = club_map.get(&owner).is_some_and(|c| c.uid == uid)
            && u32(b, at + Head::UID_AGAIN) == uid;
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
        p = at + Head::SIZE;
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
        for p in h.offset + Head::ROSTER_FROM..limit.saturating_sub(14) {
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
