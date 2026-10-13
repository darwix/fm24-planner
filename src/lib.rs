pub mod adapters;
pub mod application;
pub mod domain;
mod error;
pub mod infra;
pub mod utils;

pub use error::{Error, Result};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const APP_ID: &str = "io.github.darwix.FMSquadPlanner";
