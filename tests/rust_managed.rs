// Managed club comes from the length-prefixed club name in save_game_summary.dat.
use fm_squad_planner::adapters::parser::managed_club_id;

fn pstr(s: &str) -> Vec<u8> {
    let mut v = (s.len() as u32).to_le_bytes().to_vec();
    v.extend(s.as_bytes());
    v
}

fn summary(names: &[&str]) -> Vec<u8> {
    let mut b = vec![1, 2, 3, 0x6d, 0x01];
    for n in names {
        b.extend(pstr(n));
        b.extend([0x3c, 0x70, 0x05]);
    }
    b
}

#[test]
fn finds_club_after_league_and_manager_names() {
    let b = summary(&["Premier League", "Meong Racun", "Aston Villa"]);
    let clubs = [(603, "Huizen"), (305, "Aston Villa")];
    assert_eq!(managed_club_id(&b, &clubs), Some(305));
}

#[test]
fn ambiguous_name_gives_none() {
    let b = summary(&["Meong Racun", "Barcelona"]);
    let clubs = [(1, "Barcelona"), (2, "Barcelona")];
    assert_eq!(managed_club_id(&b, &clubs), None);
}

#[test]
fn no_match_gives_none() {
    let b = summary(&["Meong Racun", "Nowhere FC"]);
    assert_eq!(managed_club_id(&b, &[(1, "Huizen")]), None);
}
