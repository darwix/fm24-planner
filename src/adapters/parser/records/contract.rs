//! A person's contract and market value. Reverse-engineered from FM24 24.3 saves and checked
//! on a handful of players.
//!
//! The contract sits in the bytes just before the person record:
//!
//! ```text
//! expiry  [start year u16][00 00][x u32][FF x4] .. [day u16][year u16]   day at x + 12
//!         youth contracts have no start year: [FF x4][u32][00 00][day u16][year u16]
//! wage    [person id u32][team u32][00 x4][weekly wage u32][01][..][00]
//! terms   [FF x8][end day u16][end year u16][start day u16][start year u16]  before the wage row;
//!         end year 1900 means no end: a month-to-month deal
//! ```
//! The market value is not in the contract block: it is the u32 right after the person's
//! attribute block.
use super::abilities::{Ability, Linked};
use crate::adapters::parser::{
    bytes::{u16, u32},
    meta::date_of,
};
use crate::utils::cancel::{check, CancelFlag};
use crate::Result;

#[derive(Default)]
pub struct Contract {
    /// Weekly wage before tax, in the save's base currency.
    pub wage: Option<u32>,
    pub team: Option<u32>,
    pub contract_end: Option<String>,
    /// Market value in the save's base currency.
    pub value: Option<u32>,
    /// A rolling deal with no end date, which the game lists as out of contract.
    pub month_to_month: bool,
}

impl Contract {
    /// Drops the wage and end date of a contract the game no longer honours: a free agent's
    /// leftover one (team u32::MAX), one that ended before the game date (ISO dates), or a
    /// month-to-month deal.
    pub fn expire_stale(self, team: u32, game_date: &str) -> Contract {
        let ended = self
            .contract_end
            .as_deref()
            .is_some_and(|end| end < game_date);
        let rolling = self.month_to_month && self.contract_end.is_none();
        if team == u32::MAX || ended || rolling {
            Contract {
                wage: None,
                contract_end: None,
                ..self
            }
        } else {
            self
        }
    }
}

/// How far before a person record its contract can sit.
const WINDOW: usize = 800;

/// The contract and value of each linked person, in order.
pub fn for_people(db: &[u8], linked: &Linked, cancel: &CancelFlag) -> Result<Vec<Contract>> {
    let mut contracts = vec![];
    for &(person, ability) in &linked.pairs {
        check(cancel)?;
        let window = &db[person.offset.saturating_sub(WINDOW)..person.offset];
        contracts.push(Contract {
            value: value(db, ability.offset),
            ..find(window, person.id as u32)
        });
    }
    Ok(contracts)
}

/// Market value: the u32 stored right after the attribute block that starts at `ability_offset`.
pub fn value(db: &[u8], ability_offset: usize) -> Option<u32> {
    let at = ability_offset + Ability::ATTRIBUTES;
    (at + 4 <= db.len()).then(|| u32(db, at))
}

/// Wage, team and contract end from `window`, the bytes before the person record.
pub fn find(window: &[u8], person_id: u32) -> Contract {
    let Some(anchor) = wage_anchor(window, person_id) else {
        return Contract::default();
    };
    Contract {
        wage: Some(u32(window, anchor + WAGE)),
        team: Some(u32(window, anchor + TEAM)),
        contract_end: contract_end(window, anchor),
        value: None,
        month_to_month: month_to_month(window, anchor),
    }
}

/// How far before the wage row the terms block can sit.
const TERMS_RANGE: usize = 200;
/// The year a month-to-month deal stores as its end.
const NO_END_YEAR: u16 = 1900;

/// Whether the terms block nearest before the wage row has no end year.
fn month_to_month(window: &[u8], anchor: usize) -> bool {
    (anchor.saturating_sub(TERMS_RANGE)..anchor.saturating_sub(16))
        .rev()
        .find(|&at| window[at..at + 8] == [0xff; 8] && window[at + 8] != 0xff)
        .is_some_and(|at| u16(window, at + 10) == NO_END_YEAR)
}

// Wage anchor, from the person id.
const TEAM: usize = 4;
const ZEROS: usize = 8;
const WAGE: usize = 12;
/// Holds 1.
const FLAG: usize = 16;
/// Holds 0.
const FLAG_PAD: usize = 18;
const ANCHOR_LEN: usize = 19;

/// The first `[person id][team][00 x4][wage][01 ?? 00]` with a wage above zero, so a loaned
/// player reads the parent contract, not the zero-wage loan row.
fn wage_anchor(window: &[u8], person_id: u32) -> Option<usize> {
    let needle = person_id.to_le_bytes();
    (0..window.len().saturating_sub(ANCHOR_LEN)).find(|&at| {
        window[at..at + 4] == needle
            && window[at + ZEROS..at + WAGE] == [0; 4]
            && window[at + FLAG] == 1
            && window[at + FLAG_PAD] == 0
            && u32(window, at + WAGE) > 0
    })
}

// Expiry block, from `x`, the u32 after `[start year][00 00]`.
/// Bytes before `x` holding the start year.
const START_YEAR: usize = 4;
/// The FF run; a standard contract can have one more u32 before it.
const FF_RUN: usize = 4;
/// Two zero bytes before the day in a youth contract.
const YOUTH_ZEROS: usize = 10;
const DAY: usize = 12;
const YEAR: usize = 14;
/// Candidates for `x` stop this many bytes before the wage anchor.
const BEFORE_ANCHOR: usize = 11;

/// The expiry date nearest before the wage anchor. A false match can sit 4 bytes after the
/// real one, so step back while the one before also matches.
fn contract_end(window: &[u8], anchor: usize) -> Option<String> {
    let mut at = (START_YEAR..anchor.saturating_sub(BEFORE_ANCHOR))
        .rev()
        .find(|&at| expiry(window, at).is_some())?;
    while at >= 2 * START_YEAR && expiry(window, at - 4).is_some() {
        at -= 4;
    }
    expiry(window, at)
}

/// The expiry date if an expiry block has its `x` at `at`.
fn expiry(window: &[u8], at: usize) -> Option<String> {
    let (day, year) = (u16(window, at + DAY) & 511, u16(window, at + YEAR));
    let youth = youth_start(window, at);
    let start_ok = youth || standard_start(window, at);
    let ok = start_ok
        && u32(window, at) != u32::MAX // overlaps the previous candidate's FF run
        && (youth || year >= u16(window, at - START_YEAR)) // expiry never precedes the start
        && (1..=366).contains(&day)
        && (2020..=2100).contains(&year)
        && year != 2047; // 0x07FF: all-ones sentinel, not a date
    ok.then(|| date_of(year, day).ok()).flatten()
}

/// `[FF x4][u32][00 00]`: a youth contract, which has no start year.
fn youth_start(window: &[u8], at: usize) -> bool {
    window[at + FF_RUN..at + FF_RUN + 4] == [0xff; 4]
        && window[at + YOUTH_ZEROS..at + DAY] == [0; 2]
}

/// `[start year][00 00][x]` followed by the FF run, here or one u32 later.
fn standard_start(window: &[u8], at: usize) -> bool {
    (2020..=2100).contains(&u16(window, at - START_YEAR))
        && window[at - 2..at] == [0; 2]
        && (window[at + FF_RUN..at + FF_RUN + 4] == [0xff; 4]
            || window[at + FF_RUN + 4..at + FF_RUN + 8] == [0xff; 4])
}
