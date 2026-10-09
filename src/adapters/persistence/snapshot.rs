use crate::utils::cancel::CancelFlag;
use crate::{
    application::use_cases::saves::SnapshotRepository,
    domain::{
        catalog::CATALOG,
        entities::{NewSnapshot, SnapshotId, SnapshotMeta},
    },
    utils::cancel::check,
    Error, Result,
};
use rusqlite::{params_from_iter, types::Value as SqlValue, Connection, OpenFlags};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

/// Columns before the per-attribute ones, in insert order, with their SQL types.
/// The attribute columns and `detail` follow; see [`player_columns`].
const PLAYER_COLUMNS: [(&str, &str); 19] = [
    ("id", "INTEGER PRIMARY KEY"),
    ("uid", "INTEGER NOT NULL"),
    ("name", "TEXT NOT NULL"),
    ("full_name", "TEXT NOT NULL"),
    ("search_name", "TEXT NOT NULL"),
    ("age", "INTEGER"),
    ("club_id", "INTEGER"),
    ("club", "TEXT"),
    ("nation_id", "INTEGER"),
    ("nations", "TEXT NOT NULL"),
    ("nationalities", "TEXT NOT NULL"),
    ("positions", "TEXT NOT NULL"),
    ("position_mask", "INTEGER NOT NULL"),
    ("main_position", "INTEGER NOT NULL"),
    ("ca", "INTEGER"),
    ("pa", "INTEGER"),
    ("wage", "INTEGER"),
    ("contract_end", "TEXT"),
    ("value", "INTEGER"),
];

/// Every column of the players table as `name TYPE`, in insert order.
fn player_columns() -> Vec<String> {
    let fixed = PLAYER_COLUMNS
        .iter()
        .map(|(name, ty)| format!("{name} {ty}"));
    let attributes = CATALOG
        .attributes
        .iter()
        .map(|a| format!("attr_{} INTEGER", a.key));
    fixed
        .chain(attributes)
        .chain(["detail TEXT NOT NULL".to_owned()])
        .collect()
}
const INDEXES: &str = "CREATE INDEX idx_players_pa ON players(pa DESC,ca DESC,id); CREATE INDEX idx_players_ca ON players(ca); CREATE INDEX idx_players_age ON players(age); CREATE INDEX idx_players_club ON players(club_id); CREATE INDEX idx_players_nation ON players(nation_id); PRAGMA optimize;";

fn optional<T: Into<SqlValue>>(value: Option<T>) -> SqlValue {
    value.map_or(SqlValue::Null, Into::into)
}

fn create_snapshot(path: &Path, snapshot: &NewSnapshot, cancel: &CancelFlag) -> Result<()> {
    let mut db: Connection = Connection::open(path)?;
    let columns = player_columns();
    db.execute_batch(&format!(
        "CREATE TABLE metadata (key TEXT PRIMARY KEY,value TEXT NOT NULL); CREATE TABLE players ({});",
        columns.join(",")
    ))?;
    let tx = db.transaction()?;
    {
        let mut insert = tx.prepare(&format!(
            "INSERT INTO players VALUES ({})",
            vec!["?"; columns.len()].join(",")
        ))?;
        for p in &snapshot.players {
            check(cancel)?;
            let mut values: Vec<SqlValue> = vec![
                p.id.into(),
                p.uid.into(),
                p.name.clone().into(),
                p.full_name.clone().into(),
                p.search_name.clone().into(),
                p.age.into(),
                optional(p.club_id),
                optional(p.club.clone()),
                p.nation_id.into(),
                serde_json::to_string(&p.nation_ids)?.into(),
                serde_json::to_string(&p.nationalities)?.into(),
                serde_json::to_string(&p.positions)?.into(),
                p.position_mask.into(),
                (p.main_position as i64).into(),
                p.ca.into(),
                p.pa.into(),
                optional(p.wage),
                optional(p.contract_end.clone()),
                optional(p.value),
            ];
            values.extend(
                CATALOG
                    .attributes
                    .iter()
                    .map(|a| optional(p.attributes.get(&a.key).copied())),
            );
            values.push(serde_json::to_string(&p.detail)?.into());
            insert.execute(params_from_iter(values))?;
        }
    }
    tx.execute(
        "INSERT INTO metadata VALUES ('snapshot',?)",
        [serde_json::to_string(&snapshot.meta)?],
    )?;
    check(cancel)?;
    tx.commit()?;
    // Index creation is interruptible too, not only the insertion loop.
    let interrupt = db.get_interrupt_handle();
    let done = Arc::new(AtomicBool::new(false));
    std::thread::scope(|scope| -> Result<()> {
        let stopped = done.clone();
        scope.spawn(move || {
            while !stopped.load(Ordering::Relaxed) {
                if cancel.is_cancelled() {
                    interrupt.interrupt();
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        });
        let result = db.execute_batch(INDEXES);
        done.store(true, Ordering::Relaxed);
        check(cancel)?;
        result?;
        Ok(())
    })?;
    Ok(())
}
pub(super) fn open_snapshot(path: &Path) -> Result<Connection> {
    Ok(Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?)
}
fn metadata(db: &Connection) -> Result<SnapshotMeta> {
    let raw: String = db.query_row("SELECT value FROM metadata WHERE key='snapshot'", [], |r| {
        r.get(0)
    })?;
    Ok(serde_json::from_str(&raw)?)
}

/// One SQLite file per imported save, kept in `dir`.
pub struct SqliteSnapshotRepository {
    dir: PathBuf,
}
impl SqliteSnapshotRepository {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }
    fn partial(&self, job_id: &str) -> PathBuf {
        self.dir.join(format!("{job_id}.partial.sqlite"))
    }
}

/// The file of an imported snapshot.
pub(super) fn snapshot_file(dir: &Path, id: &SnapshotId) -> Result<PathBuf> {
    let path = dir.join(format!("{id}.sqlite"));
    if !path.is_file() {
        return Err(Error::query("This save has not been imported."));
    }
    Ok(path)
}

impl SnapshotRepository for SqliteSnapshotRepository {
    fn exists(&self, id: &SnapshotId) -> bool {
        snapshot_file(&self.dir, id).is_ok()
    }
    fn create(&self, job_id: &str, snapshot: &NewSnapshot, cancel: &CancelFlag) -> Result<()> {
        create_snapshot(&self.partial(job_id), snapshot, cancel)
    }
    fn publish(&self, job_id: &str, id: &SnapshotId) -> Result<()> {
        fs::rename(self.partial(job_id), self.dir.join(format!("{id}.sqlite")))?;
        Ok(())
    }
    fn unpublish(&self, id: &SnapshotId) -> Result<()> {
        fs::remove_file(self.dir.join(format!("{id}.sqlite")))?;
        Ok(())
    }
    fn discard(&self, job_id: &str) -> Result<()> {
        let partial = self.partial(job_id);
        if partial.exists() {
            fs::remove_file(partial)?;
        }
        Ok(())
    }
    fn metadata(&self, id: &SnapshotId) -> Result<SnapshotMeta> {
        metadata(&open_snapshot(&snapshot_file(&self.dir, id)?)?)
    }
}
