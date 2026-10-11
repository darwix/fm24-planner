//! A person's contract and market value. Reverse-engineered from FM24 24.3 saves and checked
//! on a handful of players.
//!
//! The contract sits in the bytes just before the person record:
//!
//! ```text
//! expiry  [start year u16][00 00][x u32][FF x4] .. [day u16][year u16]   day at x + 12
//!         youth contracts have no start year: [FF x4][u32][00 00][day u16][year u16]
//! wage    [person id u32][team u32][00 x4][weekly wage u32][01][..][00]
//! terms   [FF x4+][end day u16][end year u16][start day u16][start year u16]  before the wage row;
//!         end 1900 day 2 means no end: a month-to-month deal (1900 day 1 is the blank date)
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
use std::collections::HashMap;

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

/// How far before a person record the row with their own club can sit.
const OWN_WINDOW: usize = 3000;

/// The contract and value of each linked person, in order. `team_club` maps a team number to
/// its club, to find the row with the player's own club.
pub fn for_people(
    db: &[u8],
    linked: &Linked,
    team_club: &HashMap<u32, u32>,
    cancel: &CancelFlag,
) -> Result<Vec<Contract>> {
    let mut contracts = vec![];
    for &(person, ability) in &linked.pairs {
        check(cancel)?;
        let wide = &db[person.offset.saturating_sub(OWN_WINDOW)..person.offset];
        let id = person.id as u32;
        let own = team_club.get(&ability.team);
        let non_contract = own
            .is_some_and(|club| non_contract(wide, id, |team| team_club.get(&team) == Some(club)));
        let found = if non_contract {
            Contract::default()
        } else {
            find_wide(wide, id)
        };
        contracts.push(Contract {
            value: value(db, ability.offset),
            ..found
        });
    }
    Ok(contracts)
}

/// Contract type byte after a row's flags; 3 is a non-contract deal.
const ROW_TYPE: usize = 25;
const NON_CONTRACT: u8 = 3;

/// A non-contract deal: the row with the player's own club (nearest the person, team passes
/// `is_own`) has zero wage, FF where a paid row has its date, and either contract type 3 or
/// sits beyond the usual window with a released-style tail (FF FF ?? 00 or FF x4; Cayón B
/// youths) with no paid row before it. A loan row at the borrowing club has a date there
/// (blank, or 03 2F .. for Ortega).
// ponytail: the far-row rule is fitted on 5 players of one save (1 false hit); widen or drop it
// once more saves are checked against the game.
pub fn non_contract(window: &[u8], person_id: u32, is_own: impl Fn(u32) -> bool) -> bool {
    rows(window, person_id)
        .filter(|&at| is_own(u32(window, at + TEAM)))
        .last()
        .is_some_and(|at| {
            let far = window.len() - at > WINDOW;
            let released = window
                .get(at + ROW_DATE..at + ROW_DATE + 4)
                .is_some_and(|t| t[..2] == [0xff; 2] && (t[3] == 0 || t == [0xff; 4]));
            // a paid row before the far one is the real contract (Sirera)
            let paid_before = rows(window, person_id)
                .take_while(|&r| r < at)
                .any(|r| u32(window, r + WAGE) > 0);
            u32(window, at + WAGE) == 0
                && window.get(at + ROW_DATE) == Some(&0xff)
                && (window.get(at + ROW_TYPE) == Some(&NON_CONTRACT)
                    || (far && released && !paid_before))
        })
}

/// Market value: the u32 stored right after the attribute block that starts at `ability_offset`.
pub fn value(db: &[u8], ability_offset: usize) -> Option<u32> {
    let at = ability_offset + Ability::ATTRIBUTES;
    (at + 4 <= db.len()).then(|| u32(db, at))
}

/// `find` on the last 800 bytes of `wide`; only the month-to-month marker is looked for further
/// back, since a wage row near the window's start (Lübcke) can have it just before the window.
/// With no paid row there and no amateur deal, the whole of `wide` is read (Green: loan rows
/// first, paid rows over 800 bytes back).
pub fn find_wide(wide: &[u8], person_id: u32) -> Contract {
    let skip = wide.len().saturating_sub(WINDOW);
    let near = &wide[skip..];
    let found = find(near, person_id);
    if found.wage.is_none() && !amateur(near, person_id) {
        return find(wide, person_id);
    }
    let anchor = wage_anchor(near, person_id).map(|at| skip + at);
    let rolling = found.month_to_month || anchor.is_some_and(|at| month_to_month(wide, at));
    // a month-to-month deal has no end (an older block's would hide it); an amateur's old paid
    // row is not its contract
    let paid = found.wage.is_some() && !rolling;
    let contract_end = found
        .contract_end
        .clone()
        .or_else(|| anchor.filter(|_| paid).and_then(|at| terms_end(wide, at)));
    Contract {
        month_to_month: rolling,
        contract_end,
        ..found
    }
}

/// Wage, team and contract end from `window`, the bytes before the person record.
pub fn find(window: &[u8], person_id: u32) -> Contract {
    if amateur(window, person_id) {
        // an unset-start terms block with an end makes it a contract at no wage (Sarı), not an
        // amateur deal
        let first = rows(window, person_id).next().unwrap_or_default();
        return unset_start_end(window, first).map_or_else(Contract::default, |end| Contract {
            wage: Some(0),
            team: Some(u32(window, first + TEAM)),
            contract_end: Some(end),
            ..Contract::default()
        });
    }
    let Some(anchor) = wage_anchor(window, person_id) else {
        return Contract::default();
    };
    Contract {
        wage: Some(u32(window, anchor + WAGE)),
        team: Some(u32(window, anchor + TEAM)),
        contract_end: contract_end(window, anchor)
            .or_else(|| (!month_to_month(window, anchor)).then(|| terms_end(window, anchor))?),
        value: None,
        month_to_month: month_to_month(window, anchor),
    }
}

/// How far before the wage row the terms block can sit.
const TERMS_RANGE: usize = 450;
/// The end a month-to-month deal stores: 1900, day 2.
const NO_END: [u8; 4] = [0x02, 0x00, 0x6c, 0x07];

/// Whether a terms block before the wage row has no end but a real start. It need not be the
/// nearest block (newer dated blocks can sit between it and the row), and as few as four FF
/// bytes can lead it.
fn month_to_month(window: &[u8], anchor: usize) -> bool {
    (anchor.saturating_sub(TERMS_RANGE)..anchor.saturating_sub(12)).any(|at| {
        window[at..at + 4] == [0xff; 4]
            && window[at + 4..at + 8] == NO_END
            && (2000..=2100).contains(&u16(window, at + 10))
    })
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

/// Every `[person id][team][00 x4][wage][01 ?? 00]` row in `window`, in order.
fn rows(window: &[u8], person_id: u32) -> impl Iterator<Item = usize> + '_ {
    let needle = person_id.to_le_bytes();
    (0..window.len().saturating_sub(ANCHOR_LEN)).filter(move |&at| {
        window[at..at + 4] == needle
            && window[at + ZEROS..at + WAGE] == [0; 4]
            && window[at + FLAG] == 1
            && window[at + FLAG_PAD] == 0
    })
}

/// The first row with a wage above zero, so a loaned player reads the parent contract, not the
/// zero-wage loan row.
fn wage_anchor(window: &[u8], person_id: u32) -> Option<usize> {
    rows(window, person_id).find(|&at| u32(window, at + WAGE) > 0)
}

/// After the row's flags: 01 00 6C 07 (1900-01-01, unset) on a paid or loan row, FF on an
/// amateur one.
const ROW_DATE: usize = 19;
const BLANK_DATE: [u8; 4] = [0x01, 0x00, 0x6c, 0x07];

/// An amateur deal: the first row is zero-wage without the blank date. Any paid row after it is
/// an old contract with another club.
fn amateur(window: &[u8], person_id: u32) -> bool {
    rows(window, person_id).next().is_some_and(|at| {
        u32(window, at + WAGE) == 0
            && window.get(at + ROW_DATE..at + ROW_DATE + 4) != Some(&BLANK_DATE[..])
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

/// The expiry date nearest before `anchor` whose block reads `[1900][00 00][x = 0][FF x4]`; other
/// matches before a zero-wage row are noise (2086-01-01).
fn unset_start_end(window: &[u8], anchor: usize) -> Option<String> {
    (START_YEAR..anchor.saturating_sub(BEFORE_ANCHOR))
        .rev()
        .filter(|&at| window[at - START_YEAR..at + 4] == [0x6c, 0x07, 0, 0, 0, 0, 0, 0])
        .find_map(|at| expiry(window, at))
}

/// Every terms block reads `[04 00 00 00][u32][year or 1900][00 00][x][v1][v2][end day][end
/// year][start day][start year]`; v1 and v2 are FF when unset. The year field is not always a
/// start year (Andrés Álvarez: 2054 before an end in 2053).
const TERMS_LEN: usize = 28;
const TERMS_END: usize = 24;

/// The end in the nearest terms block before `anchor`.
fn terms_end(window: &[u8], anchor: usize) -> Option<String> {
    (0..anchor.saturating_sub(TERMS_LEN)).rev().find_map(|at| {
        let field = u16(window, at + 8);
        let (day, year) = (u16(window, at + TERMS_END) & 511, u16(window, at + TERMS_END + 2));
        let block = window[at..at + 4] == [4, 0, 0, 0]
            && (field == 1900 || (2000..=2100).contains(&field))
            && window[at + 10..at + 12] == [0; 2]
            && (1..=366).contains(&day)
            && (2020..=2100).contains(&year)
            && year != 2047;
        block.then(|| date_of(year, day).ok()).flatten()
    })
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

/// `[start year][00 00][x]` followed by the FF run, here or one u32 later. Without the run (two
/// set values instead, Duncliffe), a start date after the end must not be later than it. The
/// start year can be 1900 (unset: Simatos's youth deal, Sarı) when the FF run follows at once.
fn standard_start(window: &[u8], at: usize) -> bool {
    let (day, year) = (u16(window, at + YEAR + 2), u16(window, at + YEAR + 4));
    let start = u16(window, at - START_YEAR);
    let unset = start == 1900 && window[at + FF_RUN..at + FF_RUN + 4] == [0xff; 4];
    ((2020..=2100).contains(&start) || unset)
        && window[at - 2..at] == [0; 2]
        && (window[at + FF_RUN..at + FF_RUN + 4] == [0xff; 4]
            || window[at + FF_RUN + 4..at + FF_RUN + 8] == [0xff; 4]
            || ((1..=366).contains(&day)
                && (2000..=u16(window, at + YEAR)).contains(&year)))
}
