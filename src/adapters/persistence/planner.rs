use super::snapshot::snapshot_file;
use crate::{
    application::use_cases::planner::PlannerRepository, domain::entities::SnapshotId, Result,
};
use rusqlite::{Connection, OptionalExtension};
use serde_json::Value;
use std::{path::PathBuf, time::Duration};

/// Planner state kept in a `planner` table inside each snapshot file, created on first use.
pub struct SqlitePlannerRepository {
    dir: PathBuf,
}
impl SqlitePlannerRepository {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }
    fn open(&self, snapshot: &SnapshotId) -> Result<Connection> {
        let db = Connection::open(snapshot_file(&self.dir, snapshot)?)?;
        db.busy_timeout(Duration::from_secs(5))?;
        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS planner (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
        )?;
        Ok(db)
    }
}

impl PlannerRepository for SqlitePlannerRepository {
    fn get(&self, snapshot: &SnapshotId, key: &str) -> Result<Option<Value>> {
        let raw: Option<String> = self
            .open(snapshot)?
            .query_row("SELECT value FROM planner WHERE key=?", [key], |r| r.get(0))
            .optional()?;
        raw.map(|raw| Ok(serde_json::from_str(&raw)?)).transpose()
    }
    fn put(&self, snapshot: &SnapshotId, key: &str, value: &Value) -> Result<()> {
        self.open(snapshot)?.execute(
            "INSERT INTO planner VALUES (?1,?2) ON CONFLICT(key) DO UPDATE SET value=?2",
            [key, &serde_json::to_string(value)?],
        )?;
        Ok(())
    }
}
