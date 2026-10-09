// Private, reverse-engineered from FM24 24.3 saves; verified on a handful of players.
// Contract block sits just before the person record:
//   [start yr u16][00 00][X u32][FF FF FF FF] ... [expiry day u16][expiry yr u16]   (expiry = X + 12)
//   [eid u32][team u32][00 x4][weekly wage u32][01 ?? 00] ...                        (wage anchor)
use super::date_of;

#[derive(Default)]
pub struct Contract {
    pub wage: Option<u32>,
    pub contract_end: Option<String>,
}

fn u16(b: &[u8], p: usize) -> u16 {
    u16::from_le_bytes([b[p], b[p + 1]])
}
fn u32(b: &[u8], p: usize) -> u32 {
    u32::from_le_bytes([b[p], b[p + 1], b[p + 2], b[p + 3]])
}

/// `window` is the bytes before the person record. The first anchor with a wage wins, so a
/// loaned player reads the parent contract, not the zero-wage loan row.
pub fn find(window: &[u8], eid: u32) -> Contract {
    let needle = eid.to_le_bytes();
    let anchor = (0..window.len().saturating_sub(19))
        .filter(|&i| window[i..i + 4] == needle)
        .filter(|&i| window[i + 8..i + 12] == [0; 4] && window[i + 16] == 1 && window[i + 18] == 0)
        .find(|&i| u32(window, i + 12) > 0);
    let Some(i) = anchor else {
        return Contract::default();
    };
    let contract_end = (4..i.saturating_sub(11)).rev().find_map(|q| {
        let (day, year) = (u16(window, q + 12) & 511, u16(window, q + 14));
        let ok = (2020..=2100).contains(&u16(window, q - 4))
            && window[q - 2..q] == [0; 2]
            && (window[q + 4..q + 8] == [0xff; 4] || window[q + 8..q + 12] == [0xff; 4]) // an extra u32 can precede the run
            && u32(window, q) != u32::MAX // overlaps the previous candidate's FF run
            && year >= u16(window, q - 4) // expiry never precedes the start
            && (1..=366).contains(&day)
            && (2020..=2100).contains(&year)
            && year != 2047; // 0x07FF: all-ones sentinel, not a date
        ok.then(|| date_of(year, day).ok()).flatten()
    });
    Contract {
        wage: Some(u32(window, i + 12)),
        contract_end,
    }
}

/// Transfer value in the save's base currency: the u32 right after the 54-byte attribute block.
pub fn value(b: &[u8], ability_offset: usize) -> Option<u32> {
    let p = ability_offset + 54;
    (p + 4 <= b.len()).then(|| u32(b, p))
}
