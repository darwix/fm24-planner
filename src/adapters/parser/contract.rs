// Private, reverse-engineered from FM24 24.3 saves; verified on a handful of players.
// Contract block sits just before the person record:
//   [start yr u16][00 00][X u32][FF FF FF FF] ... [expiry day u16][expiry yr u16]   (expiry = X + 12)
//   [eid u32][team u32][00 x4][weekly wage u32][01 ?? 00] ...                        (wage anchor)
use super::abilities::Ability;
use super::bytes::{u16, u32};
use super::save::date_of;

#[derive(Default)]
pub struct Contract {
    pub wage: Option<u32>,
    pub team: Option<u32>,
    pub contract_end: Option<String>,
}

// Wage anchor, offsets from the start of `[eid]`.
const TEAM: usize = 4;
const ZEROS: usize = 8;
const WAGE: usize = 12;
const FLAG: usize = 16; // == 1
const FLAG_PAD: usize = 18; // == 0
const ANCHOR_LEN: usize = 19;
// Expiry block, offsets from `q`, the u32 `X` that follows `[start yr][00 00]`.
const FF_RUN: usize = 4; // youth: FF run here; standard: here or one u32 later
const DAY: usize = 12;
const YEAR: usize = 14;

/// `window` is the bytes before the person record. The first anchor with a wage wins, so a
/// loaned player reads the parent contract, not the zero-wage loan row.
pub fn find(window: &[u8], eid: u32) -> Contract {
    let needle = eid.to_le_bytes();
    let anchor = (0..window.len().saturating_sub(ANCHOR_LEN))
        .filter(|&i| window[i..i + 4] == needle)
        .filter(|&i| {
            window[i + ZEROS..i + WAGE] == [0; 4]
                && window[i + FLAG] == 1
                && window[i + FLAG_PAD] == 0
        })
        .find(|&i| u32(window, i + WAGE) > 0);
    let Some(i) = anchor else {
        return Contract::default();
    };
    let expiry = |q: usize| {
        let (day, year) = (u16(window, q + DAY) & 511, u16(window, q + YEAR));
        // Youth contracts have no start year: [FF x4][u32][day][year] instead of [start yr][00 00][X].
        let youth = window[q + FF_RUN..q + FF_RUN + 4] == [0xff; 4]
            && window[q + DAY - 2..q + DAY] == [0; 2];
        let ok = (youth
            || (2020..=2100).contains(&u16(window, q - 4))
                && window[q - 2..q] == [0; 2]
                && (window[q + FF_RUN..q + FF_RUN + 4] == [0xff; 4] || window[q + FF_RUN + 4..q + FF_RUN + 8] == [0xff; 4])) // an extra u32 can precede the run
            && u32(window, q) != u32::MAX // overlaps the previous candidate's FF run
            && (youth || year >= u16(window, q - 4)) // expiry never precedes the start
            && (1..=366).contains(&day)
            && (2020..=2100).contains(&year)
            && year != 2047; // 0x07FF: all-ones sentinel, not a date
        ok.then(|| date_of(year, day).ok()).flatten()
    };
    // A false match can sit 4 bytes after the real one, so step back while the earlier one also matches.
    let contract_end = (4..i.saturating_sub(11))
        .rev()
        .find(|&q| expiry(q).is_some())
        .and_then(|mut q| {
            while q >= 8 && expiry(q - 4).is_some() {
                q -= 4;
            }
            expiry(q)
        });
    Contract {
        wage: Some(u32(window, i + WAGE)),
        team: Some(u32(window, i + TEAM)),
        contract_end,
    }
}

/// Transfer value in the save's base currency: the u32 right after the attribute block.
pub fn value(b: &[u8], ability_offset: usize) -> Option<u32> {
    let p: usize = ability_offset + Ability::ATTRIBUTES;
    (p + 4 <= b.len()).then(|| u32(b, p))
}
