// Private reverse-engineering tests. Fixtures are the 800 bytes before a player's
// person record in an FM24 save; expected values are what the game showed.
use fm_savelens_backend::adapters::parser::records::contract::{find, value};

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

#[test]
fn contract_team_field_is_read() {
    let c = find(&bytes(include_str!("fixtures/pietrangeli.hex")), 448321);
    assert_eq!(c.team, Some(432));
    let c = find(&bytes(include_str!("fixtures/krznar.hex")), 529029);
    assert_eq!(c.team, Some(1298));
}

#[test]
fn youth_contract_has_no_start_year_before_the_expiry() {
    check(
        include_str!("fixtures/strand.hex"),
        638149,
        220,
        "2053-06-30",
    );
}

// Duncliffe (Arsenal): full time, £5,110 p/w, expires 2053-06-30; was read as expired.
#[test]
fn duncliffe_wage_and_expiry() {
    check(
        include_str!("fixtures/duncliffe.hex"),
        550333,
        5_108,
        "2053-06-30",
    );
}

// Green: on loan at West Brom (£2,550) from Liverpool (£25,540, expires 2053-06-30). Both paid
// rows sit before the last 800 bytes, behind three zero-wage rows. Fixture is 3000 bytes.
#[test]
fn green_paid_rows_before_the_window() {
    use fm_savelens_backend::adapters::parser::records::contract::find_wide;
    let c = find_wide(&bytes(include_str!("fixtures/green.hex")), 515186);
    assert_eq!(c.wage, Some(25_541));
    assert_eq!(c.contract_end.as_deref(), Some("2053-06-30"));
}

// Simatos (Tottenham): youth contract, expires 2053-06-30; the block's start year is 1900 (unset).
#[test]
fn simatos_unset_start_year() {
    check(
        include_str!("fixtures/simatos.hex"),
        637770,
        220,
        "2053-06-30",
    );
}

// Sarı (Bodø/Glimt): full time at £0 until 2054-12-31. The zero-wage row looks amateur, but the
// terms block holds an end (start year 1900, unset).
#[test]
fn sari_zero_wage_contract_with_an_end() {
    check(include_str!("fixtures/sari.hex"), 657818, 0, "2054-12-31");
}

// Terms blocks all read [04 00 00 00][u32][year][00 00][x][v1][v2][end day][end year][start].
// Miki (Newcastle, to 2057-06-30): his block sits before the last 800 bytes. Fixture: 3000 bytes.
#[test]
fn terms_block_before_the_window() {
    use fm_savelens_backend::adapters::parser::records::contract::find_wide;
    let c = find_wide(&bytes(include_str!("fixtures/terms_far.hex")), 584929);
    assert_eq!(c.wage, Some(32_819));
    assert_eq!(c.contract_end.as_deref(), Some("2057-06-30"));
}

// Andrés Álvarez (Inter de Palmira, to 2053-12-31): the year field before x (2054) is later
// than the end, so it is not a start year.
#[test]
fn terms_block_with_a_later_year_field() {
    use fm_savelens_backend::adapters::parser::records::contract::find_wide;
    let c = find_wide(&bytes(include_str!("fixtures/terms_late_year.hex")), 246717);
    assert_eq!(c.wage, Some(5_797));
    assert_eq!(c.contract_end.as_deref(), Some("2053-12-31"));
}

// Player 584177: his FF-dated contract row (£22,030, to 2053-06-30) sits 1190 bytes back; the
// four blank-dated paid rows after it are other clubs' approaches, not his contract.
#[test]
fn blank_dated_approach_rows_are_skipped() {
    use fm_savelens_backend::adapters::parser::records::contract::find_wide;
    let c = find_wide(&bytes(include_str!("fixtures/approaches.hex")), 584177);
    assert_eq!(c.wage, Some(22_030));
    assert_eq!(c.contract_end.as_deref(), Some("2053-06-30"));
}
