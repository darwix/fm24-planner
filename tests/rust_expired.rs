// Contracts the game no longer honours must not reach the snapshot: a free agent's leftover
// contract (Matthias Ziegler, FrA in-game, decoded £307/week to 2054) and one already past.
use fm_savelens_backend::adapters::parser::records::contract::Contract;

fn contract(end: &str) -> Contract {
    Contract {
        wage: Some(307),
        team: Some(732),
        contract_end: Some(end.into()),
        value: Some(1000),
        month_to_month: false,
    }
}

#[test]
fn free_agent_drops_wage_and_end_but_keeps_value() {
    let c = contract("2054-06-30").expire_stale(u32::MAX, "2053-03-08");
    assert_eq!((c.wage, c.contract_end, c.value), (None, None, Some(1000)));
}

#[test]
fn contract_ended_before_the_game_date_is_dropped() {
    let c = contract("2052-06-30").expire_stale(17761, "2053-03-08");
    assert_eq!((c.wage, c.contract_end), (None, None));
}

#[test]
fn running_contract_is_kept() {
    let c = contract("2053-03-08").expire_stale(17761, "2053-03-08");
    assert_eq!(
        (c.wage, c.contract_end.as_deref()),
        (Some(307), Some("2053-03-08"))
    );
}

// Month-to-month (Radim Kaufman, Vitkovice, 375 a week since 2049-08-01): the
// [FF x8][end][start] block before the wage row holds end year 1900, i.e. none. The game lists
// these players as out of contract.
#[test]
fn month_to_month_contract_counts_as_expired() {
    use fm_savelens_backend::adapters::parser::records::contract::find;
    let id: u32 = 550384;
    let mut w = vec![0u8; 100];
    w.extend([0xff; 8]);
    w.extend([0x02, 0x00, 0x6c, 0x07, 0xd5, 0x54, 0x01, 0x08]); // end 1900, start 2049-08-01
    w.extend([0u8; 40]);
    w.extend(id.to_le_bytes());
    w.extend(829u32.to_le_bytes());
    w.extend([0; 4]);
    w.extend(376u32.to_le_bytes());
    w.extend([1, 0, 0]);
    w.extend([0u8; 30]);
    let c = find(&w, id);
    assert!(c.month_to_month);
    let c = c.expire_stale(829, "2053-03-08");
    assert_eq!((c.wage, c.contract_end), (None, None));
}

// An amateur deal (Airam, Los Llanos): the first contract row is the current club's at zero
// wage, and an old paid contract with another club follows. The game lists him as out of
// contract, so the old wage must not count. A loanee's first row is the paid parent contract
// (see rust_contract.rs, Ruiz), so this does not affect loans.
#[test]
fn zero_wage_first_row_means_no_paid_contract() {
    use fm_savelens_backend::adapters::parser::records::contract::find;
    let id: u32 = 605612;
    let row = |team: u32, wage: u32| {
        let mut r = id.to_le_bytes().to_vec();
        r.extend(team.to_le_bytes());
        r.extend([0; 4]);
        r.extend(wage.to_le_bytes());
        r.extend([1, 0, 0]);
        r.extend([0xff, 0x03, 0, 0]);
        r.extend([0u8; 60]);
        r
    };
    let mut w = vec![0u8; 100];
    w.extend(row(22540, 0));
    w.extend(row(8070, 268));
    let c = find(&w, id);
    assert_eq!((c.wage, c.contract_end), (None, None));
}

// Mitch Brannigan (Watford, on loan): his first row is also zero-wage, but it carries a blank
// date (01 00 6C 07 = 1900-01-01) where an amateur row has FF; his paid contract still counts.
#[test]
fn zero_wage_row_with_a_blank_date_keeps_the_paid_contract() {
    use fm_savelens_backend::adapters::parser::records::contract::find;
    let id: u32 = 700001;
    let row = |team: u32, wage: u32, tail: [u8; 4]| {
        let mut r = id.to_le_bytes().to_vec();
        r.extend(team.to_le_bytes());
        r.extend([0; 4]);
        r.extend(wage.to_le_bytes());
        r.extend([1, 0, 0]);
        r.extend(tail);
        r.extend([0u8; 60]);
        r
    };
    let mut w = vec![0u8; 100];
    w.extend(row(500, 0, [0x01, 0x00, 0x6c, 0x07]));
    w.extend(row(431, 2923, [0x01, 0x00, 0x6c, 0x07]));
    assert_eq!(find(&w, id).wage, Some(2923));
}

// Austin Moses (Oldham, month-to-month): the end-1900 block is not the nearest one; a newer
// dated block (2050-01-02 / 2050-01-20) sits between it and the wage row.
#[test]
fn month_to_month_block_need_not_be_the_nearest() {
    use fm_savelens_backend::adapters::parser::records::contract::find;
    let id: u32 = 530570;
    let mut w = vec![0u8; 100];
    w.extend([0xff; 8]);
    w.extend([0x02, 0x00, 0x6c, 0x07, 0xc3, 0x00, 0x01, 0x08]); // end 1900, start 2049
    w.extend([0u8; 20]);
    w.extend([0xff; 8]);
    w.extend([0x02, 0x00, 0x02, 0x08, 0x14, 0x00, 0x02, 0x08]); // 2050-01-02 / 2050-01-20
    w.extend([0u8; 30]);
    w.extend(id.to_le_bytes());
    w.extend(1300u32.to_le_bytes());
    w.extend([0; 4]);
    w.extend(300u32.to_le_bytes());
    w.extend([1, 0, 0, 0x01, 0x00, 0x6c, 0x07]);
    w.extend([0u8; 30]);
    assert!(find(&w, id).month_to_month);
}

// Eyal Sitton (Maccabi Tel-Aviv, month-to-month): the end-1900 block sits about 340 bytes
// before the wage row.
#[test]
fn month_to_month_block_can_sit_far_before_the_row() {
    use fm_savelens_backend::adapters::parser::records::contract::find;
    let id: u32 = 107923;
    let mut w = vec![0u8; 100];
    w.extend([0xff; 8]);
    w.extend([0x02, 0x00, 0x6c, 0x07, 0x44, 0x00, 0xf6, 0x07]); // end 1900, start 2038
    w.extend([0u8; 325]);
    w.extend(id.to_le_bytes());
    w.extend(840u32.to_le_bytes());
    w.extend([0; 4]);
    w.extend(202u32.to_le_bytes());
    w.extend([1, 0, 0]);
    w.extend([0u8; 30]);
    assert!(find(&w, id).month_to_month);
}
