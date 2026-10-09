pub mod player;
mod save_files;
mod settings;
pub mod snapshot;

pub use player::SqlitePlayerRepository;
pub use save_files::FsSaveFiles;
pub use settings::JsonSettingsRepository;
pub use snapshot::SqliteSnapshotRepository;

impl From<rusqlite::Error> for crate::Error {
    fn from(e: rusqlite::Error) -> Self {
        Self::new("DATABASE", e.to_string())
    }
}
