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
            "CREATE TABLE IF NOT EXISTS planner (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS shortlist (player_id INTEGER PRIMARY KEY);
             -- once: the old planner `kept` list held player copies; keep only their ids
             INSERT OR IGNORE INTO shortlist
               SELECT json_extract(j.value, '$.id') FROM planner, json_each(planner.value) j
               WHERE planner.key = 'kept';
             DELETE FROM planner WHERE key = 'kept';",
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
    fn shortlist(&self, snapshot: &SnapshotId) -> Result<Vec<u32>> {
        let db = self.open(snapshot)?;
        let mut stmt = db.prepare("SELECT player_id FROM shortlist ORDER BY player_id")?;
        let ids = stmt
            .query_map([], |r| r.get(0))?
            .collect::<rusqlite::Result<Vec<u32>>>()?;
        Ok(ids)
    }
    fn shortlist_add(&self, snapshot: &SnapshotId, player: u32) -> Result<()> {
        self.open(snapshot)?
            .execute("INSERT OR IGNORE INTO shortlist VALUES (?)", [player])?;
        Ok(())
    }
    fn shortlist_remove(&self, snapshot: &SnapshotId, player: u32) -> Result<()> {
        self.open(snapshot)?
            .execute("DELETE FROM shortlist WHERE player_id = ?", [player])?;
        Ok(())
    }
    fn put(&self, snapshot: &SnapshotId, key: &str, value: &Value) -> Result<()> {
        self.open(snapshot)?.execute(
            "INSERT INTO planner VALUES (?1,?2) ON CONFLICT(key) DO UPDATE SET value=?2",
            [key, &serde_json::to_string(value)?],
        )?;
        Ok(())
    }
}
