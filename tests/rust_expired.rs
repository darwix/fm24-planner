// Contracts the game no longer honours must not reach the snapshot: a free agent's leftover
// contract (Matthias Ziegler, FrA in-game, decoded £307/week to 2054) and one already past.
use fm_savelens_backend::adapters::parser::records::contract::Contract;

fn contract(end: &str) -> Contract {
    Contract {
        wage: Some(307),
        team: Some(732),
        contract_end: Some(end.into()),
        value: Some(1000),
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
    assert_eq!((c.wage, c.contract_end.as_deref()), (Some(307), Some("2053-03-08")));
}
