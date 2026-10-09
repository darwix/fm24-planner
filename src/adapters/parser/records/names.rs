//! Name pools: the first, last and common (nick) names every person record points into.
//!
//! ```text
//! pool  = [count u32] then count entries
//! entry = [id u32][len u32][text]      ids run 0, 1, 2, ...
//! ```
//! The three pools sit back to back. People follow them; clubs and teams come before.
use super::{
    super::bytes::u32,
    scan::{checkpoint, text},
};
use crate::{
    utils::cancel::{check, CancelFlag},
    Error, Result,
};

const POOL_COUNT: usize = 4;
const POOL_ENTRY: usize = 8;

pub struct Names {
    pub first: Vec<String>,
    pub last: Vec<String>,
    pub common: Vec<String>,
    /// Where the first-name pool starts.
    pub start: usize,
    /// Where the common-name pool ends.
    pub end: usize,
}
impl Names {
    /// Name `id` in `pool`, if the id is in range.
    pub(super) fn pick(pool: &[String], id: u32) -> Option<&str> {
        pool.get(id as usize).map(String::as_str)
    }
}

/// Finds the three name pools: the first offset whose pools all read cleanly.
pub fn find(db: &[u8], cancel: &CancelFlag) -> Result<Names> {
    for offset in 0..db.len().saturating_sub(40) {
        checkpoint(offset, cancel)?;
        if !looks_like_first_names(db, offset) {
            continue;
        }
        let Some((first, end)) = pool(db, offset, cancel)? else {
            continue;
        };
        let Some((last, end)) = pool(db, end, cancel)? else {
            continue;
        };
        if last.len() < 10000 {
            continue;
        }
        let Some((common, end)) = pool(db, end, cancel)? else {
            continue;
        };
        return Ok(Names {
            first,
            last,
            common,
            start: offset,
            end,
        });
    }
    Err(Error::new(
        "UNSUPPORTED_NAMES",
        "Could not locate the FM24 name tables in this save.",
    ))
}

/// Cheap test before reading a whole pool: a large count, then entry 0 followed by entry 1.
fn looks_like_first_names(db: &[u8], offset: usize) -> bool {
    let entry = offset + POOL_COUNT;
    if db[entry..entry + 4] != [0; 4] || !(10000..=3_000_000).contains(&u32(db, offset)) {
        return false;
    }
    let len = u32(db, entry + 4) as usize;
    let next = entry + POOL_ENTRY + len;
    (1..=100).contains(&len) && next + 4 <= db.len() && u32(db, next) == 1
}

/// Reads the pool at `start`: its names and where it ends.
fn pool(db: &[u8], start: usize, cancel: &CancelFlag) -> Result<Option<(Vec<String>, usize)>> {
    if start + POOL_COUNT + POOL_ENTRY + 4 >= db.len() {
        return Ok(None);
    }
    let count = u32(db, start) as usize;
    if !(1..=3_000_000).contains(&count) {
        return Ok(None);
    }
    let mut names = Vec::new();
    let mut offset = start + POOL_COUNT;
    for id in 0..count {
        if id % 4096 == 0 {
            check(cancel)?;
        }
        if offset + POOL_ENTRY > db.len() || u32(db, offset) as usize != id {
            return Ok(None);
        }
        let len = u32(db, offset + 4) as usize;
        let name = offset + POOL_ENTRY;
        if len > 512 || name + len > db.len() {
            return Ok(None);
        }
        let Some(s) = text(&db[name..name + len]) else {
            return Ok(None);
        };
        names.push(s);
        offset = name + len;
    }
    Ok(Some((names, offset)))
}
