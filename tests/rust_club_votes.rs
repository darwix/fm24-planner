// A team whose own record names no club still has players whose contract names one (Fram's 19432).
use fm_savelens_backend::adapters::parser::teams::club_by_contract;
use std::collections::HashSet;

fn clubs() -> HashSet<u32> {
    [641, 672].into()
}

#[test]
fn team_takes_the_club_most_of_its_players_are_contracted_to() {
    let rows = [(19432, 641), (19432, 641), (19432, 672)];
    assert_eq!(club_by_contract(&rows, &clubs())[&19432], 641);
}

#[test]
fn contract_values_that_are_not_clubs_are_ignored() {
    let rows = [(31315, 31316), (31315, 31316)];
    assert!(club_by_contract(&rows, &clubs()).is_empty());
}

#[test]
fn a_tie_picks_no_club() {
    let rows = [(7, 641), (7, 672)];
    assert!(club_by_contract(&rows, &clubs()).is_empty());
}
