use super::{create_snapshot, metadata, open_snapshot, player, search};
use crate::{application::use_cases::ports::SnapshotStore, domain::entities::ParsedSave, Result};
use serde_json::Value;
use std::path::Path;
use tokio_util::sync::CancellationToken;

/// One SQLite file per imported save.
pub struct SqliteSnapshots;

impl SnapshotStore for SqliteSnapshots {
    fn create(
        &self,
        path: &Path,
        save: &ParsedSave,
        meta: Value,
        cancel: &CancellationToken,
    ) -> Result<()> {
        create_snapshot(path, save, meta, cancel)
    }
    fn metadata(&self, path: &Path) -> Result<Value> {
        metadata(&open_snapshot(path)?)
    }
    fn player(&self, path: &Path, id: u32) -> Result<Value> {
        player(&open_snapshot(path)?, id)
    }
    fn search(&self, path: &Path, query: &str) -> Result<Value> {
        search(&open_snapshot(path)?, query)
    }
}
