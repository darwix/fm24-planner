// Club records: [id][uid][uid][00][nation][FF x4][u32][nation again] .. [name len][name]...
use fm_squad_planner::{adapters::parser::records::clubs, utils::cancel::CancelFlag};

fn record(nations: [u32; 3], name: &str) -> Vec<u8> {
    let mut b = vec![0u8; 8];
    b.extend(22583u32.to_le_bytes());
    b.extend(0x044a_a2a0u32.to_le_bytes());
    b.extend(0x044a_a2a0u32.to_le_bytes());
    b.push(0);
    b.extend(nations[0].to_le_bytes());
    b.extend([0xff; 4]);
    b.extend(nations[1].to_le_bytes());
    b.extend(nations[2].to_le_bytes());
    b.extend([0x91, 0x24, 0, 0, 0, 0x0a, 0, 0, 0, 0]);
    for _ in 0..2 {
        b.extend((name.len() as u32).to_le_bytes());
        b.extend(name.as_bytes());
    }
    b.extend([1u8; 100]);
    b
}

fn names(db: &[u8]) -> Vec<String> {
    clubs::find(db, db.len(), &CancelFlag::new())
        .unwrap()
        .into_iter()
        .map(|c| c.name)
        .collect()
}

#[test]
fn a_club_with_one_nation_is_found() {
    assert_eq!(names(&record([120, 120, 120], "Seattle Sounders FC")), ["Seattle Sounders FC"]);
}

// Toronto FC, a Canadian club in the US league: the first nation (120) differs from the
// repeated one (97).
#[test]
fn a_club_playing_in_another_nations_league_is_found() {
    assert_eq!(names(&record([120, 97, 97], "Toronto FC")), ["Toronto FC"]);
}
