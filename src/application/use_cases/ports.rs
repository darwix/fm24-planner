use crate::{domain::entities::ParsedSave, Result};
use serde_json::Value;
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

/// Stores imported saves as snapshot files.
pub trait SnapshotRepository: Send + Sync {
    fn create(
        &self,
        path: &Path,
        save: &ParsedSave,
        meta: Value,
        cancel: &CancellationToken,
    ) -> Result<()>;
    fn metadata(&self, path: &Path) -> Result<Value>;
}

/// Reads players from an imported snapshot.
pub trait PlayerRepository: Send + Sync {
    fn player(&self, path: &Path, id: u32) -> Result<Value>;
    fn search(&self, path: &Path, query: &str) -> Result<Value>;
}
