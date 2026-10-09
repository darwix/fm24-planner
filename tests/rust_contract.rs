// Private reverse-engineering tests. Fixtures are the 800 bytes before a player's
// person record in an FM24 save; expected values are what the game showed.
use fm_savelens_backend::parser::contract::{find, value};

fn bytes(hex: &str) -> Vec<u8> {
    hex.split_whitespace()
        .map(|b| u8::from_str_radix(b, 16).unwrap())
        .collect()
}

fn check(hex: &str, eid: u32, wage: u32, end: &str) {
    let c = find(&bytes(hex), eid);
    assert_eq!(c.wage, Some(wage));
    assert_eq!(c.contract_end.as_deref(), Some(end));
}

#[test]
fn pietrangeli_wage_and_expiry() {
    check(
        include_str!("fixtures/pietrangeli.hex"),
        448321,
        455_280,
        "2056-06-30",
    );
}

#[test]
fn krznar_wage_and_expiry() {
    check(
        include_str!("fixtures/krznar.hex"),
        529029,
        229_641,
        "2057-06-30",
    );
}

#[test]
fn ziglam_wage_and_expiry() {
    check(
        include_str!("fixtures/ziglam.hex"),
        637473,
        3_520,
        "2054-06-30",
    );
}

#[test]
fn loaned_player_reads_the_parent_contract_not_the_zero_wage_loan_row() {
    check(
        include_str!("fixtures/ruiz.hex"),
        499757,
        39_146,
        "2054-06-30",
    );
}

#[test]
fn unknown_eid_gives_nothing() {
    let c = find(&bytes(include_str!("fixtures/pietrangeli.hex")), 1);
    assert_eq!(c.wage, None);
    assert_eq!(c.contract_end, None);
}

#[test]
fn value_is_the_u32_54_bytes_after_the_ability_block() {
    let mut b = vec![0u8; 100];
    b[10 + 54..10 + 58].copy_from_slice(&96_391_200u32.to_le_bytes());
    assert_eq!(value(&b, 10), Some(96_391_200));
    assert_eq!(value(&b, 90), None);
}

#[test]
fn year_2047_sentinel_is_not_an_expiry() {
    check(
        include_str!("fixtures/fornies.hex"),
        567917,
        17_508,
        "2056-06-30",
    );
}

// Soldi: a false match one u32 later (start = low half of X, X = FFFFFFFF) decoded 2050-03-20.
#[test]
fn soldi_ignores_overlapping_false_expiry() {
    check(
        include_str!("fixtures/soldi.hex"),
        621367,
        25_886,
        "2057-06-30",
    );
}

// Planchart: an extra u32 sits before the FF run, and a false match one field later has start 2100 > end 2050.
#[test]
fn planchart_extra_field_before_ff_run() {
    check(
        include_str!("fixtures/planchart.hex"),
        381134,
        8_238,
        "2054-06-30",
    );
}

// Dawson: the false match sits 4 bytes after the real one (start 2032 reads as a year); the earlier one wins.
#[test]
fn dawson_prefers_the_earlier_of_two_overlapping_matches() {
    check(
        include_str!("fixtures/dawson.hex"),
        550711,
        21_890,
        "2056-06-30",
    );
}
