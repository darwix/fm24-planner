use fm_savelens_backend::parser::archive::Archive;
use tokio_util::sync::CancellationToken;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let c = CancellationToken::new();
    let mut ar = Archive::open(std::path::Path::new(&a[1]), &c).unwrap();
    std::fs::write(&a[3], ar.member(&a[2], &c).unwrap()).unwrap();
}
