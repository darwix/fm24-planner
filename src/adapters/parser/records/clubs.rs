//! Club records, before the name pools.
//!
//! ```text
//! [id u32][uid u32][uid u32][00][nation u32][FF x4] .. [nation u32] .. [name len u32]   43 bytes
//! [name][short name len u32][short name]
//! ```
use super::{
    super::bytes::u32,
    scan::{is_set, scan, text},
};
use crate::{domain::entities::Club, utils::cancel::CancelFlag, Result};

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
