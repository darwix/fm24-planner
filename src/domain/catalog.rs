use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

/// A player attribute the game tracks, as the save orders them.
#[derive(Deserialize, Serialize)]
pub struct Attribute {
    pub key: String,
    pub label: String,
    pub group: String,
    pub index: usize,
    pub scale: u8,
    pub inverted: bool,
}
/// The attributes and positions every player is rated on.
#[derive(Deserialize, Serialize)]
pub struct Catalog {
    pub attributes: Vec<Attribute>,
    pub positions: Vec<String>,
}
pub static CATALOG: LazyLock<Catalog> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../assets/attributes.json")).expect("attribute catalog")
});
