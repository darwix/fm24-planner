// private diagnostic: why does a player with a contract have no club? read-only.
use fm_squad_planner::adapters::parser::{archive::Archive, bytes, records};
use std::{collections::HashMap, path::Path};
use fm_squad_planner::utils::cancel::CancelFlag;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let c = CancelFlag::new();
    let mut ar = Archive::open(Path::new(&a[1]), &c).unwrap();
    let b = ar.member("game_db.dat", &c).unwrap();
    let names = records::names::find(&b, &c).unwrap();
    let people = records::people::find(&b, &names, &c).unwrap();
    let blocks = records::abilities::find(&b, names.end, &c).unwrap();
    let clubs = records::clubs::find(&b, names.start, &c).unwrap();
    let valid = people
        .iter()
        .filter(|p| p.id >= 0)
        .map(|p| p.id as u32)
        .collect();
    let teams = records::teams::find(&b, &clubs, names.start, &valid, &c).unwrap();
    let tm: HashMap<_, _> = teams.iter().map(|t| (t.ordinal + 1, t)).collect();
    if let Ok(o) = std::env::var("ORD") {
        let o: u32 = o.parse().unwrap();
        println!(
            "clubs {}; club 20935: {:?}",
            clubs.len(),
            clubs
                .iter()
                .find(|c| c.id == 20935)
                .map(|c| (&c.name, c.uid))
        );
        for at in 4..names.start - 70 {
            if bytes::u32(&b, at + 14) == o
                && bytes::u32(&b, at + 22) == bytes::u32(&b, at + 18)
            {
                println!(
                    "loose head at {at} owner {} b26={} pad={:?}",
                    bytes::u32(&b, at),
                    b[at + 26],
                    &b[at + 4..at + 14]
                );
            }
        }
        for at in 4..names.start - 70 {
            if bytes::u32(&b, at + 14) == o && b[at + 26] == 10 && b[at + 4..at + 14] == [0; 10] {
                println!(
                    "head at {at} owner {} uid {}",
                    bytes::u32(&b, at),
                    bytes::u32(&b, at + 18)
                );
                for (i, ch) in b[at - 8..at + 40].chunks(16).enumerate() {
                    println!(
                        "{:>8} {}",
                        at - 8 + i * 16,
                        ch.iter()
                            .map(|x| format!("{x:02x}"))
                            .collect::<Vec<_>>()
                            .join(" ")
                    );
                }
            }
        }
        return;
    }
    if let Ok(o) = std::env::var("NEAR") {
        let o: u32 = o.parse().unwrap();
        for t in teams
            .iter()
            .filter(|t| t.ordinal + 20 >= o && t.ordinal <= o + 20)
        {
            println!(
                "ord {} kind {} club {:?} members {}",
                t.ordinal,
                t.kind,
                t.club,
                t.members.len()
            );
        }
        return;
    }
    for id in &a[2..] {
        let id: u64 = id.parse().unwrap();
        let ab = blocks.iter().find(|x| x.person_id == id).unwrap();
        let pe = people.iter().find(|p| p.id == id as i64).unwrap();
        let w = &b[pe.offset.saturating_sub(800)..pe.offset];
        let needle = (id as u32).to_le_bytes();
        let ct: Vec<u32> = (0..w.len() - 19)
            .filter(|&i| {
                w[i..i + 4] == needle
                    && w[i + 8..i + 12] == [0; 4]
                    && w[i + 16] == 1
                    && w[i + 18] == 0
            })
            .map(|i| bytes::u32(w, i + 4))
            .collect();
        println!(
            "  contract team fields {ct:?}; club_by_id {:?}",
            ct.iter()
                .map(|t| clubs
                    .iter()
                    .find(|c| c.id == *t)
                    .map(|c| c.short_name.clone()))
                .collect::<Vec<_>>()
        );
        let t = tm.get(&ab.team);
        let inmem: Vec<_> = teams
            .iter()
            .filter(|t| t.members.contains(&(id as u32)))
            .map(|t| (t.ordinal, t.kind, t.club))
            .collect();
        println!(
            "{id} team {} -> {:?}; member of {:?}",
            ab.team,
            t.map(|t| (t.ordinal, t.kind, t.club, t.members.len())),
            inmem
        );
    }
}
