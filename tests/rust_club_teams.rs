// A club record lists its teams after its names: [-1 or a club id][00 00 00][count][count team
// numbers], the first team first (AS Vitré: 3148, 20055, 34615).
use fm_savelens_backend::{
    adapters::parser::records::{clubs::team_lists, teams::Team},
    domain::entities::Club,
};

fn record(id: u32, name: &str, teams: &[u32]) -> (Vec<u8>, Club) {
    record_after(id, name, &[7u8; 20], teams)
}

/// A club record whose team list comes after `between` (the record's other fields).
fn record_after(id: u32, name: &str, between: &[u8], teams: &[u32]) -> (Vec<u8>, Club) {
    let mut b = vec![0u8; 39];
    b.extend((name.len() as u32).to_le_bytes());
    b.extend(name.as_bytes());
    b.extend((name.len() as u32).to_le_bytes());
    b.extend(name.as_bytes());
    b.extend(between);
    b.extend([0xff; 4]);
    b.extend([0, 0, 0, teams.len() as u8]);
    for t in teams {
        b.extend(t.to_le_bytes());
    }
    b.extend([0u8; 1200]);
    let club = Club {
        id,
        uid: 1,
        name: name.into(),
        short_name: name.into(),
        nation: 0,
        offset: 0,
    };
    (b, club)
}

fn concat(parts: Vec<(Vec<u8>, Club)>) -> (Vec<u8>, Vec<Club>) {
    let (mut db, mut clubs) = (vec![], vec![]);
    for (b, mut c) in parts {
        c.offset = db.len();
        db.extend(b);
        clubs.push(c);
    }
    (db, clubs)
}

#[test]
fn each_listed_team_maps_to_its_club() {
    let (db, clubs) = concat(vec![
        record(2910, "AS Vitré", &[3148, 20055, 34615]),
        record(650, "Beitar", &[829, 31620]),
    ]);
    let lists = team_lists(&db, &clubs, &[]);
    assert_eq!(
        (lists[&3148], lists[&34615], lists[&829]),
        (2910, 2910, 650)
    );
}

#[test]
fn a_team_listed_by_two_clubs_is_left_out() {
    let (db, clubs) = concat(vec![
        record(1, "One", &[500]),
        record(2, "Two", &[500, 501]),
    ]);
    let lists = team_lists(&db, &clubs, &[]);
    assert!(!lists.contains_key(&500));
    assert_eq!(lists[&501], 2);
}

// A. Getxo: zeros then [00 00 00 01][824] is not a list; id 0 never marks one, though the save
// has a club record with id 0.
#[test]
fn club_id_zero_does_not_mark_a_list() {
    let mut between = vec![0u8; 12];
    between.extend([0, 0, 0, 1]);
    between.extend(824u32.to_le_bytes());
    between.extend([7u8; 12]);
    let (db, clubs) = concat(vec![
        record_after(1834, "A. Getxo", &between, &[2036, 33474]),
        record(0, "Zero", &[9]),
    ]);
    let lists = team_lists(&db, &clubs, &[]);
    assert!(!lists.contains_key(&824));
    assert_eq!(lists[&33474], 1834);
}

// A. Getxo's list sits 18 KB after its names, past an unread record; it still counts up to the
// next club record.
#[test]
fn a_list_far_after_the_names_counts_up_to_the_next_club() {
    let (db, clubs) = concat(vec![
        record_after(1834, "A. Getxo", &[7u8; 18000], &[2036, 33474]),
        record(1836, "Raa IF", &[2040]),
    ]);
    let lists = team_lists(&db, &clubs, &[]);
    assert_eq!((lists[&33474], lists[&2040]), (1834, 1836));
}

// S. Gijón B: a club id then [00 00 00 01][30] comes before the real FF-marked list; the FF one
// wins.
#[test]
fn an_ff_marked_list_beats_an_earlier_club_marked_one() {
    let mut between = 5u32.to_le_bytes().to_vec();
    between.extend([0, 0, 0, 1]);
    between.extend(30u32.to_le_bytes());
    between.extend([7u8; 12]);
    let (db, clubs) = concat(vec![
        record_after(1106, "S. Gijon B", &between, &[1295, 1296]),
        record(5, "Five", &[9]),
    ]);
    let lists = team_lists(&db, &clubs, &[]);
    assert!(!lists.contains_key(&30));
    assert_eq!(lists[&1295], 1106);
}

// Aurillac's list [1464, 32537, 32538] has neither FF x4 nor a club id before it, while a false
// [1465] earlier does. The list whose first team's own record names this club wins.
#[test]
fn a_list_whose_first_team_names_the_club_wins() {
    let mut between = u32::MAX.to_le_bytes().to_vec();
    between.extend([0, 0, 0, 1]);
    between.extend(1465u32.to_le_bytes());
    between.extend(0x500fu32.to_le_bytes());
    between.extend([0, 0, 0, 3]);
    between.extend(1464u32.to_le_bytes());
    between.extend(32537u32.to_le_bytes());
    between.extend(32538u32.to_le_bytes());
    between.extend([7u8; 12]);
    let (db, clubs) = concat(vec![record_after(1263, "Aurillac FC", &between, &[9])]);
    let first_team = Team {
        club: Some(1263),
        ordinal: 1463,
        kind: 100,
        members: vec![],
    };
    let lists = team_lists(&db, &clubs, &[first_team]);
    assert!(!lists.contains_key(&1465));
    assert_eq!((lists[&1464], lists[&32538]), (1263, 1263));
}

// Team number to club: a club's own list beats the club a team record names.
#[test]
fn team_clubs_prefers_the_club_lists() {
    use fm_savelens_backend::adapters::parser::records::teams::team_clubs;
    let teams = [
        Team {
            club: Some(4198),
            ordinal: 29220,
            kind: 100,
            members: vec![],
        },
        Team {
            club: Some(650),
            ordinal: 828,
            kind: 100,
            members: vec![],
        },
    ];
    let map = team_clubs(&teams, &[(29221, 27877)].into());
    assert_eq!((map[&29221], map[&829]), (27877, 650));
}
