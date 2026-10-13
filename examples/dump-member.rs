use fm_squad_planner::adapters::parser::archive::Archive;
use fm_squad_planner::utils::cancel::CancelFlag;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let c = CancelFlag::new();
    let mut ar = Archive::open(std::path::Path::new(&a[1]), &c).unwrap();
    std::fs::write(&a[3], ar.member(&a[2], &c).unwrap()).unwrap();
}
