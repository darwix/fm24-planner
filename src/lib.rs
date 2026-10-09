pub mod adapters;
pub mod domain;
pub mod error;
pub mod services;

pub use adapters::{parser, storage};
pub use domain::{hybrid, rating_systems, roles};
pub use error::{Error, Result};
pub use services as service;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const APP_ID: &str = "io.github.MSalman5230.FMSaveLens24";
