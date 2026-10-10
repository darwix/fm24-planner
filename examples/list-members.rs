use fm_savelens_backend::adapters::parser::archive::Archive;
use fm_savelens_backend::utils::cancel::CancelFlag;
fn main() {
    let p = std::env::args().nth(1).unwrap();
    let a = Archive::open(std::path::Path::new(&p), &CancelFlag::new()).unwrap();
    for m in a.members() {
        println!("{:>12} {:>12} {}", m.stored, m.plain, m.name);
    }
}
