// A team whose own record names no club still has players whose contract names one (Fram's 19432).
use fm_savelens_backend::adapters::parser::records::teams::club_by_contract;
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

// Free agents carry team u32::MAX; leftover contract fields must not vote them into a club.
#[test]
fn free_agents_get_no_club_from_votes() {
    use fm_savelens_backend::adapters::parser::records::teams::player_clubs;
    let free = u32::MAX;
    let players = [(1, free, Some(641)), (2, free, Some(641)), (3, free, None)];
    assert_eq!(player_clubs(&[], &players, &clubs()), vec![None; 3]);
}
