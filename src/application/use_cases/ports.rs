use crate::{
    domain::entities::{
        NewSnapshot, ParsedSave, PlayerDetail, PlayerFilter, PlayerPage, Settings, SnapshotId,
        SnapshotMeta, SourceFile,
    },
    Result,
};
use std::path::Path;
use tokio_util::sync::CancellationToken;

/// Reads a `.fm` save file into domain entities.
pub trait SaveReader: Send + Sync {
    /// Changes whenever parser output changes, so cached snapshots are rebuilt.
    fn version(&self) -> &str;
    fn read(
        &self,
        path: &Path,
        cancel: &CancellationToken,
        progress: &dyn Fn(u8, &str),
    ) -> Result<ParsedSave>;
}

/// Looks at `.fm` files and folders on disk.
pub trait SaveFiles: Send + Sync {
    /// The `.fm` files directly inside `folder`.
    fn scan(&self, folder: &str) -> Result<Vec<SourceFile>>;
    fn stat(&self, path: &Path) -> Result<SourceFile>;
    /// The canonical path of an existing folder.
    fn directory(&self, folder: &str) -> Result<String>;
}

/// Keeps the user settings between runs.
pub trait SettingsRepository: Send + Sync {
    fn load(&self) -> Result<Settings>;
    fn save(&self, settings: &Settings) -> Result<()>;
}

/// Stores imported saves as snapshots. An import is written under its job id
/// first and published under the snapshot id once it is complete.
pub trait SnapshotRepository: Send + Sync {
    fn exists(&self, id: &SnapshotId) -> bool;
    fn create(
        &self,
        job_id: &str,
        snapshot: &NewSnapshot,
        cancel: &CancellationToken,
    ) -> Result<()>;
    fn publish(&self, job_id: &str, id: &SnapshotId) -> Result<()>;
    fn unpublish(&self, id: &SnapshotId) -> Result<()>;
    /// Drops the unpublished write of `job_id`, if any.
    fn discard(&self, job_id: &str) -> Result<()>;
    fn metadata(&self, id: &SnapshotId) -> Result<SnapshotMeta>;
}

/// Reads players from an imported snapshot.
pub trait PlayerRepository: Send + Sync {
    fn player(&self, snapshot: &SnapshotId, id: u32) -> Result<Option<PlayerDetail>>;
    fn search(&self, snapshot: &SnapshotId, filter: &PlayerFilter) -> Result<PlayerPage>;
}
