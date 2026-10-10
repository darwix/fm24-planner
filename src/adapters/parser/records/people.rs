//! Person records, right after the name pools.
//!
//! ```text
//! head  [first u32][00][last u32][00][common u32][00][name len u32, top bytes 00 00]   19 bytes
//! name  [full name]                                                                     name len
//! tail  [birth day u16][birth year u16] .. [nation u16][00 x6][personality x8]          25 bytes
//!       .. [01 if a list follows][count][count other-nationality entries of 16 bytes]
//! ```
//! The person record holds no id. Ids come from identity records further on: the longest
//! ascending chain of them is taken, and each person gets the first identity after it.
use super::{
    super::bytes::{u16, u32},
    names::Names,
    scan::{checkpoint, is_set, scan, text},
};
use crate::{
    utils::cancel::{check, CancelFlag},
    Error, Result,
};
use std::collections::HashSet;

pub struct Person {
    pub offset: usize,
    /// Where the full name ends and the tail starts.
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

/// The nickname (common name) wins when the player has one, else first + last.
pub fn display_name(first: &str, last: &str, common: Option<&str>) -> String {
    match common {
        Some(c) if !c.is_empty() => c.to_string(),
        _ => format!("{first} {last}").trim().to_string(),
    }
}

/// The name a nickname index points at: the common pool when it fits, the person's own first
/// name when it repeats the first index (regens like "Francisco Miguel"), else the last-name
/// pool ("Juanma"). u32::MAX is out of every pool, so no nickname.
pub fn nickname(names: &Names, first: u32, common: u32) -> Option<&str> {
    match common {
        c if (c as usize) < names.common.len() => Names::pick(&names.common, c),
        c if c == first => Names::pick(&names.first, c),
        c => Names::pick(&names.last, c),
    }
}

/// Finds every person record, then links each to its identity and other nationalities.
pub fn find(db: &[u8], names: &Names, cancel: &CancelFlag) -> Result<Vec<Person>> {
    let mut people = scan(names.end, db.len().saturating_sub(100), cancel, |offset| {
        let person = read_person(db, names, offset)?;
        let next = person.end + Tail::SIZE;
        Some((person, next))
    })?;
    if names.end + 4 > db.len() {
        return Err(Error::new("TRUNCATED", "The person table is incomplete."));
    }
    let identities = identities(db, names.end, cancel)?;
    let chain = ascending_chain(&identities, cancel)?;
    link_identities(db, &mut people, &chain, cancel)?;
    Ok(people)
}

struct Head {
    first: u32,
    last: u32,
    common: u32,
    name_len: usize,
}
impl Head {
    const SIZE: usize = 19;
    /// None unless the separator bytes are zero.
    fn read(db: &[u8], offset: usize) -> Option<Self> {
        let zero = |i: usize| db[offset + i] == 0;
        (zero(4) && zero(9) && zero(14) && zero(17) && zero(18)).then(|| Self {
            first: u32(db, offset),
            last: u32(db, offset + 5),
            common: u32(db, offset + 10),
            name_len: u32(db, offset + 15) as usize,
        })
    }
}

struct Tail {
    birth_day: u16,
    birth_year: u16,
    nation: u16,
    personality: [u8; 8],
}
impl Tail {
    const SIZE: usize = 25;
    /// Bytes the scan needs past the name before it trusts a match.
    const MIN_ROOM: usize = 45;
    /// `1` when an other-nationality list follows.
    const NATIONS_FLAG: usize = 33;
    const NATIONS_COUNT: usize = 34;
    const NATIONS: usize = 35;
    /// None unless the six bytes before the personality are zero.
    fn read(db: &[u8], offset: usize) -> Option<Self> {
        (db[offset + 11..offset + 17] == [0; 6]).then(|| Self {
            birth_day: u16(db, offset),
            birth_year: u16(db, offset + 2),
            nation: u16(db, offset + 9),
            personality: db[offset + 17..offset + 25].try_into().unwrap(),
        })
    }
    fn plausible(&self) -> bool {
        (1..=366).contains(&self.birth_day)
            && (1850..=2300).contains(&self.birth_year)
            && self.nation <= 255
            && self.personality.iter().all(|v| (1..=20).contains(v))
    }
}

/// The person record at `offset`, without its id yet.
fn read_person(db: &[u8], names: &Names, offset: usize) -> Option<Person> {
    let head = Head::read(db, offset)?;
    if (head.first != u32::MAX && head.first as usize >= names.first.len())
        || (head.last != u32::MAX && head.last as usize >= names.last.len())
        || head.name_len > 200
    {
        return None;
    }
    let end = offset + Head::SIZE + head.name_len;
    if end + Tail::MIN_ROOM > db.len() {
        return None;
    }
    let tail = Tail::read(db, end).filter(Tail::plausible)?;
    let full_name = text(&db[offset + Head::SIZE..end])?;
    let name = Some(display_name(
        Names::pick(&names.first, head.first).unwrap_or(""),
        Names::pick(&names.last, head.last).unwrap_or(""),
        nickname(names, head.first, head.common),
    ))
    .filter(|n| !n.is_empty())
    .unwrap_or_else(|| full_name.clone());
    if name.is_empty() {
        return None;
    }
    Some(Person {
        offset,
        end,
        id: -1,
        uid: u32::MAX,
        full_name: if full_name.is_empty() {
            name.clone()
        } else {
            full_name
        },
        name,
        birth_day: tail.birth_day,
        birth_year: tail.birth_year,
        nation: tail.nation,
        personality: tail.personality.to_vec(),
        other_nations: vec![],
        identity_offset: 0,
    })
}

struct Identity {
    offset: usize,
    id: u32,
    uid: u32,
}

/// Bytes from an identity's id that [`identity`] reads.
const IDENTITY_SIZE: usize = 12;

/// Every identity after the name pools. The u32 at `names_end` is the highest person id.
fn identities(db: &[u8], names_end: usize, cancel: &CancelFlag) -> Result<Vec<Identity>> {
    let max_id = u32(db, names_end) as u64 + 1;
    let mut found = Vec::new();
    for offset in names_end + 7..db.len().saturating_sub(IDENTITY_SIZE) {
        checkpoint(offset, cancel)?;
        found.extend(identity(db, offset, max_id));
    }
    Ok(found)
}

/// Identity at `offset`: `[id u32][uid u32][uid again, or a source id u32]`, preceded by a
/// type byte and three zero bytes. When the uid is not repeated, a birth date and flags sit
/// just before it.
fn identity(db: &[u8], offset: usize, max_id: u64) -> Option<Identity> {
    let p = offset;
    if db[p - 1] != 0
        || db[p - 2] != 0
        || db[p - 3] != 0
        || db[p - 7] & 7 > 2
        || ![0, 1, 4, 5].contains(&db[p - 4])
    {
        return None;
    }
    let (id, uid, source) = (u32(db, p), u32(db, p + 4), u32(db, p + 8));
    if id as u64 >= max_id || !is_set(uid) {
        return None;
    }
    if uid != source {
        if p < 12 {
            return None;
        }
        let (day, year, flags) = (u16(db, p - 12) & 511, u16(db, p - 10), db[p - 6]);
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
    if id & 255 == 0 && uid & 255 == 0 && u32(db, p + 5) == u32(db, p + 9) {
        return None;
    }
    Some(Identity { offset, id, uid })
}

/// Longest chain of strictly ascending ids, in file order, including tie replacement.
/// Real identities ascend; stray matches fall out of the chain.
fn ascending_chain<'a>(
    identities: &'a [Identity],
    cancel: &CancelFlag,
) -> Result<Vec<&'a Identity>> {
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
    let mut chain = vec![];
    let mut ix = indices.last().copied();
    while let Some(i) = ix {
        chain.push(&identities[i]);
        ix = prev[i];
    }
    chain.reverse();
    Ok(chain)
}

/// Gives each person the first chained identity after its tail and before the next person,
/// then reads its other nationalities, which sit between the two.
fn link_identities(
    db: &[u8],
    people: &mut [Person],
    chain: &[&Identity],
    cancel: &CancelFlag,
) -> Result<()> {
    let mut j = 0;
    for i in 0..people.len() {
        if i % 4096 == 0 {
            check(cancel)?;
        }
        let next_person = people.get(i + 1).map_or(db.len(), |p| p.offset);
        let person = &mut people[i];
        while j < chain.len() && chain[j].offset < person.end + Tail::SIZE {
            j += 1;
        }
        let Some(identity) = chain.get(j).filter(|x| x.offset < next_person) else {
            continue;
        };
        person.id = identity.id as i64;
        person.uid = identity.uid;
        person.identity_offset = identity.offset;
        person.other_nations = other_nations(db, person, identity.offset);
    }
    Ok(())
}

/// One 16-byte entry of the other-nationality list: `[nation u32][00 x4] .. [08][09 or 70]`.
const OTHER_NATION_SIZE: usize = 16;

fn other_nations(db: &[u8], person: &Person, limit: usize) -> Vec<u32> {
    let count = db[person.end + Tail::NATIONS_COUNT] as usize;
    let list = person.end + Tail::NATIONS;
    let mut nations = vec![];
    if db[person.end + Tail::NATIONS_FLAG] != 1 || list + count * OTHER_NATION_SIZE > limit {
        return nations;
    }
    for k in 0..count {
        let at = list + k * OTHER_NATION_SIZE;
        let nation = u32(db, at);
        let entry = nation < 256
            && db[at + 4..at + 8] == [0; 4]
            && db[at + 10] == 8
            && [9, 70].contains(&db[at + 11]);
        if entry && nation != person.nation as u32 && !nations.contains(&nation) {
            nations.push(nation);
        }
    }
    nations
}

/// Ids of the people an identity was found for.
pub fn ids(people: &[Person]) -> HashSet<u32> {
    people
        .iter()
        .filter(|p| p.id >= 0)
        .map(|p| p.id as u32)
        .collect()
}
