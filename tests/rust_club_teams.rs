// A club record lists its teams after its names: [-1 or a club id][00 00 00][count][count team
// numbers], the first team first (AS Vitré: 3148, 20055, 34615).
use fm_savelens_backend::{adapters::parser::records::clubs::team_lists, domain::entities::Club};

fn record(id: u32, name: &str, teams: &[u32]) -> (Vec<u8>, Club) {
    let mut b = vec![0u8; 39];
    b.extend((name.len() as u32).to_le_bytes());
    b.extend(name.as_bytes());
    b.extend((name.len() as u32).to_le_bytes());
    b.extend(name.as_bytes());
    b.extend([7u8; 20]); // other fields
    b.extend([0xff; 4]);
    b.extend([0, 0, 0, teams.len() as u8]);
    for t in teams {
        b.extend(t.to_le_bytes());
    }
    b.extend([0u8; 1200]);
    let club = Club { id, uid: 1, name: name.into(), short_name: name.into(), nation: 0, offset: 0 };
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
    let (db, clubs) = concat(vec![record(2910, "AS Vitré", &[3148, 20055, 34615]), record(650, "Beitar", &[829, 31620])]);
    let lists = team_lists(&db, &clubs);
    assert_eq!((lists[&3148], lists[&34615], lists[&829]), (2910, 2910, 650));
}

#[test]
fn a_team_listed_by_two_clubs_is_left_out() {
    let (db, clubs) = concat(vec![record(1, "One", &[500]), record(2, "Two", &[500, 501])]);
    let lists = team_lists(&db, &clubs);
    assert!(!lists.contains_key(&500));
    assert_eq!(lists[&501], 2);
}
