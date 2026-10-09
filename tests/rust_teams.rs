// Reserve/youth team records carry their own name instead of the ten zero bytes; they must still link to a club.
use fm_savelens_backend::adapters::parser::records::{teams, Club};
use std::collections::HashSet;
use tokio_util::sync::CancellationToken;

fn pstr(s: &str) -> Vec<u8> {
    let mut v = (s.len() as u32).to_le_bytes().to_vec();
    v.extend(s.as_bytes());
    v.push(0);
    v
}

fn club(id: u32, name: &str) -> Club {
    Club {
        id,
        uid: 1000 + id,
        name: name.into(),
        short_name: name.into(),
        nation: 0,
        offset: 0,
    }
}

// one reserve-team record followed by a squad of players 1 and 2
fn save(owner: u32, name: &str) -> Vec<u8> {
    let mut b = vec![0u8; 100];
    b.extend([0x00, 0x10, 0x00, 0x00, 0x64, 0xff]);
    b.extend(owner.to_le_bytes());
    b.extend(pstr(name));
    b.extend(pstr(name));
    b.extend(500u32.to_le_bytes()); // ordinal
    b.extend(7u32.to_le_bytes()); // uid
    b.extend(7u32.to_le_bytes());
    b.push(10);
    b.extend([0u8; 4]);
    b.extend([0xff; 4]);
    b.extend(2u16.to_le_bytes());
    for id in [1u32, 2] {
        b.extend(id.to_le_bytes());
    }
    b.extend([0xff; 8]); // captain, vice
    b.extend([0u8; 100]);
    b
}

fn run(b: &[u8], clubs: &[Club]) -> Vec<(Option<u32>, u32, Vec<u32>)> {
    let valid: HashSet<u32> = [1, 2].into();
    teams(b, clubs, b.len(), &valid, &CancellationToken::new())
        .unwrap()
        .into_iter()
        .map(|t| (t.club, t.ordinal, t.members))
        .collect()
}

#[test]
fn reserve_team_links_to_parent_by_name() {
    let b = save(125716, "OGC Nice 2");
    let got = run(&b, &[club(490, "OGC Nice")]);
    assert_eq!(got, vec![(Some(490), 500, vec![1, 2])]);
}

#[test]
fn reserve_team_links_to_club_before_owner() {
    let b = save(226, "Sparta Prague B");
    let got = run(&b, &[club(225, "Sparta Prague")]);
    assert_eq!(got, vec![(Some(225), 500, vec![1, 2])]);
}

// Under-20 team of a big club (owner id <= 256): flag 20 means the club is owner - 1.
#[test]
fn flag_20_team_with_small_owner_links_to_club_before_owner() {
    let mut b = vec![0u8; 100];
    b.extend([0x01, 0x64, 0xff, 0x14]);
    b.extend(18u32.to_le_bytes());
    b.extend([0u8; 10]);
    b.extend(500u32.to_le_bytes());
    b.extend(7u32.to_le_bytes());
    b.extend(7u32.to_le_bytes());
    b.push(10);
    b.extend([0u8; 4]);
    b.extend([0xff; 4]);
    b.extend(2u16.to_le_bytes());
    for id in [1u32, 2] {
        b.extend(id.to_le_bytes());
    }
    b.extend([0xff; 8]);
    b.extend([0u8; 100]);
    let got = run(&b, &[club(17, "Huracan"), club(18, "Independiente")]);
    assert_eq!(got, vec![(Some(17), 500, vec![1, 2])]);
}
