pub mod abilities;
pub mod archive;
pub mod bytes;
pub mod clubs;
pub mod contract;
pub mod names;
pub mod people;
pub mod reader;
pub mod save;
mod scan;
pub mod teams;

pub use save::{managed_club_id, parse_save, PARSER_VERSION};
