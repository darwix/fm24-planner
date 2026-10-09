use super::nation_name;
use crate::{
    application::use_cases::ports::SnapshotRepository,
    domain::entities::SnapshotId,
    parser::{archive::check, ParsedSave, CATALOG, PARSER_VERSION},
    utils::text::normalize,
    Error, Result,
};
use chrono::Utc;
use rusqlite::{params_from_iter, types::Value as SqlValue, Connection, OpenFlags};
use serde_json::{json, Value};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};
use tokio_util::sync::CancellationToken;

pub fn create_snapshot(
    path: &Path,
    save: &ParsedSave,
    mut meta: Value,
    cancel: &CancellationToken,
) -> Result<()> {
    let mut db: Connection = Connection::open(path)?;
    let attributes: String = CATALOG
        .attributes
        .iter()
        .map(|a| format!("attr_{} INTEGER", a.key))
        .collect::<Vec<_>>()
        .join(",");
    db.execute_batch(&format!("CREATE TABLE metadata (key TEXT PRIMARY KEY,value TEXT NOT NULL); CREATE TABLE players (
        id INTEGER PRIMARY KEY,uid INTEGER NOT NULL,name TEXT NOT NULL,full_name TEXT NOT NULL,search_name TEXT NOT NULL,
        age INTEGER,club_id INTEGER,club TEXT,nation_id INTEGER,nations TEXT NOT NULL,positions TEXT NOT NULL,position_mask INTEGER NOT NULL,main_position INTEGER NOT NULL,
        ca INTEGER,pa INTEGER,wage INTEGER,contract_end TEXT,value INTEGER,{attributes},detail TEXT NOT NULL);"))?;
    let tx = db.transaction()?;
    {
        let placeholders = vec!["?"; 19 + CATALOG.attributes.len()].join(",");
        let mut insert = tx.prepare(&format!("INSERT INTO players VALUES ({placeholders})"))?;
        for p in &save.players {
            check(cancel)?;
            let mut nations = vec![p.nation_id as u32];
            nations.extend(&p.other_nation_ids);
            let mut detail = serde_json::to_value(p)?;
            detail["nationalities"] = json!(nations
                .iter()
                .map(|&id| nation_name(id))
                .collect::<Vec<_>>());
            let mut values: Vec<SqlValue> = vec![
                (p.id as i64).into(),
                (p.uid as i64).into(),
                p.name.clone().into(),
                p.full_name.clone().into(),
                normalize(&format!("{} {} {}", p.name, p.full_name, p.uid)).into(),
                (p.age as i64).into(),
                p.club_id.map_or(SqlValue::Null, |id| (id as i64).into()),
                p.club.clone().map_or(SqlValue::Null, SqlValue::Text),
                (p.nation_id as i64).into(),
                serde_json::to_string(&nations)?.into(),
                serde_json::to_string(&p.positions)?.into(),
                p.position_ratings
                    .iter()
                    .enumerate()
                    .fold(
                        0i64,
                        |mask, (i, v)| if *v >= 15 { mask | (1 << i) } else { mask },
                    )
                    .into(),
                // best-rated position (first on ties); the stored `positions` list is catalog order
                (p.position_ratings
                    .iter()
                    .enumerate()
                    .fold(
                        (0usize, 0u8),
                        |best, (i, v)| if *v > best.1 { (i, *v) } else { best },
                    )
                    .0 as i64)
                    .into(),
                (p.ca as i64).into(),
                (p.pa as i64).into(),
                p.wage.map_or(SqlValue::Null, |v| (v as i64).into()),
                p.contract_end
                    .clone()
                    .map_or(SqlValue::Null, SqlValue::Text),
                // a value of 0 means the save has none, so keep it unknown
                p.value
                    .filter(|v| *v > 0)
                    .map_or(SqlValue::Null, |v| (v as i64).into()),
            ];
            values.extend(CATALOG.attributes.iter().map(|a| {
                p.attributes
                    .get(&a.key)
                    .map_or(SqlValue::Null, |v| (*v as i64).into())
            }));
            values.push(serde_json::to_string(&detail)?.into());
            insert.execute(params_from_iter(values))?;
        }
    }
    let ids: BTreeSet<u32> = save
        .players
        .iter()
        .flat_map(|p| std::iter::once(p.nation_id as u32).chain(p.other_nation_ids.iter().copied()))
        .collect();
    let mut nations: Vec<_> = ids
        .into_iter()
        .map(|id| json!({"id":id,"name":nation_name(id)}))
        .collect();
    nations.sort_by_key(|v| normalize(v["name"].as_str().unwrap_or("")));
    let used: BTreeSet<_> = save.players.iter().filter_map(|p| p.club_id).collect();
    let mut clubs: Vec<_> = save
        .clubs
        .iter()
        .filter(|c| used.contains(&c.id))
        .map(|c| json!({"id":c.id,"name":c.short_name}))
        .collect();
    clubs.sort_by_key(|v| normalize(v["name"].as_str().unwrap_or("")));
    let fields = json!({"name":save.name,"gameDate":save.game_date,"formatVersion":save.format_version,"playerCount":save.players.len(),"diagnostics":save.diagnostics,"warnings":save.warnings,"importedAt":Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis,true),"nations":nations,"clubs":clubs,"managedClubId":save.managed_club_id});
    meta.as_object_mut()
        .ok_or_else(|| Error::query("Invalid snapshot metadata."))?
        .extend(fields.as_object().unwrap().clone());
    tx.execute(
        "INSERT INTO metadata VALUES ('snapshot',?)",
        [serde_json::to_string(&meta)?],
    )?;
    check(cancel)?;
    tx.commit()?;
    // Index creation is interruptible too, not only the insertion loop.
    let interrupt = db.get_interrupt_handle();
    let done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    std::thread::scope(|scope| -> Result<()> {
        let stopped = done.clone();
        scope.spawn(move || {
            while !stopped.load(std::sync::atomic::Ordering::Relaxed) {
                if cancel.is_cancelled() {
                    interrupt.interrupt();
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        });
        let result = db.execute_batch("CREATE INDEX idx_players_pa ON players(pa DESC,ca DESC,id); CREATE INDEX idx_players_ca ON players(ca); CREATE INDEX idx_players_age ON players(age); CREATE INDEX idx_players_club ON players(club_id); CREATE INDEX idx_players_nation ON players(nation_id); PRAGMA optimize;");
        done.store(true, std::sync::atomic::Ordering::Relaxed);
        check(cancel)?;
        result?;
        Ok(())
    })?;
    Ok(())
}
pub fn open_snapshot(path: &Path) -> Result<Connection> {
    Ok(Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?)
}
pub fn metadata(db: &Connection) -> Result<Value> {
    let raw: String = db.query_row("SELECT value FROM metadata WHERE key='snapshot'", [], |r| {
        r.get(0)
    })?;
    let mut m: Value = serde_json::from_str(&raw)?;
    m["stale"] = json!(
        std::fs::metadata(m["sourcePath"].as_str().unwrap_or("")).map_or(true, |s| {
            s.len() != m["sourceSize"].as_u64().unwrap_or(0)
                || crate::utils::files::mtime(&s).ok() != m["sourceMtime"].as_f64()
                || m["parserVersion"].as_str() != Some(PARSER_VERSION)
        })
    );
    Ok(m)
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
    fn create(
        &self,
        job_id: &str,
        save: &ParsedSave,
        meta: Value,
        cancel: &CancellationToken,
    ) -> Result<()> {
        create_snapshot(&self.partial(job_id), save, meta, cancel)
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
    fn metadata(&self, id: &SnapshotId) -> Result<Value> {
        metadata(&open_snapshot(&snapshot_file(&self.dir, id)?)?)
    }
}
