pub mod adapters;
pub mod application;
pub mod domain;
pub mod infra;
pub mod utils;

pub use adapters::{parser, persistence as storage};
pub use application::{Error, Result};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const APP_ID: &str = "io.github.MSalman5230.FMSaveLens24";
