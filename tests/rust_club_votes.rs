// A team whose own record names no club still has players whose contract names one (Fram's 19432).
use fm_squad_planner::adapters::parser::records::teams::club_by_contract;
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
    use fm_squad_planner::adapters::parser::records::teams::player_clubs;
    let free = u32::MAX;
    let players = [(1, free, Some(641)), (2, free, Some(641)), (3, free, None)];
    assert_eq!(
        player_clubs(&[], &players, &clubs(), &Default::default()),
        vec![None; 3]
    );
}

// A contract's team field is a team ordinal + 1, not a club id. Richard Laisney's squad (team
// 19459, no record) votes 650: team ordinal 649 is Niort (471), while club id 650 is Beitar.
#[test]
fn contract_team_field_resolves_through_the_team_ordinal() {
    use fm_squad_planner::adapters::parser::records::teams::{player_clubs, Team};
    let niort = Team {
        club: Some(471),
        ordinal: 649,
        kind: 100,
        members: vec![],
    };
    let clubs = [471, 650].into();
    let players = [
        (1, 19459, Some(650)),
        (2, 19459, Some(650)),
        (3, 19459, Some(19459)),
    ];
    assert_eq!(
        player_clubs(&[niort], &players, &clubs, &Default::default()),
        vec![Some(471); 3]
    );
}

// A club record lists its teams; that beats a roster the scanner attached to the wrong team.
// AS Vitré (2910) lists team 3148, whose record is not found, so its squad landed on
// Guarany's team (3145).
#[test]
fn club_team_list_wins_over_a_misattached_roster() {
    use fm_squad_planner::adapters::parser::records::teams::{player_clubs, Team};
    let guarany = Team {
        club: Some(2909),
        ordinal: 3145,
        kind: 100,
        members: vec![1],
    };
    let listed = [(3148, 2910)].into();
    let clubs = [2909, 2910].into();
    assert_eq!(
        player_clubs(&[guarany], &[(1, 3148, None)], &clubs, &listed),
        vec![Some(2910)]
    );
}

// A player whose own team is none (u32::MAX) is a free agent in-game, even when a roster still
// holds him (US Ansoise trialists).
#[test]
fn free_agent_on_a_roster_has_no_club() {
    use fm_squad_planner::adapters::parser::records::teams::{player_clubs, Team};
    let ansoise = Team {
        club: Some(5095),
        ordinal: 7000,
        kind: 100,
        members: vec![1],
    };
    let players = [(1, u32::MAX, None)];
    assert_eq!(
        player_clubs(&[ansoise], &players, &[5095].into(), &Default::default()),
        vec![None]
    );
}
