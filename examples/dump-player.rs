// private reverse-engineering helper: dump raw bytes around one player. read-only.
use fm_squad_planner::adapters::parser::{archive::Archive, records};
use std::path::Path;
use fm_squad_planner::utils::cancel::CancelFlag;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (path, id) = (&a[1], a[2].parse::<i64>().unwrap());
    let c = CancelFlag::new();
    let mut ar = Archive::open(Path::new(path), &c).unwrap();
    let b = ar.member("game_db.dat", &c).unwrap();
    let names = records::names::find(&b, &c).unwrap();
    let people = records::people::find(&b, &names, &c).unwrap();
    let blocks = records::abilities::find(&b, names.end, &c).unwrap();
    let p = people.iter().find(|p| p.id == id).expect("no person");
    let ab = blocks
        .iter()
        .find(|x| x.person_id == id as u64)
        .expect("no ability");
    println!(
        "person {}..{} identity {} ability {}",
        p.offset, p.end, p.identity_offset, ab.offset
    );
    let (mut s, mut e) = (p.offset, (ab.offset + 54).max(p.identity_offset + 64));
    if a.len() > 4 {
        s = a[3].parse().unwrap();
        e = a[4].parse().unwrap();
    }
    for (i, ch) in b[s..e.min(b.len())].chunks(16).enumerate() {
        println!(
            "{:>8} {}",
            s + i * 16,
            ch.iter()
                .map(|x| format!("{x:02x}"))
                .collect::<Vec<_>>()
                .join(" ")
        );
    }
}
