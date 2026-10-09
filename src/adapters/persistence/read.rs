use crate::{parser::PARSER_VERSION, Error, Result};
use rusqlite::{Connection, OpenFlags};
use serde_json::{json, Value};
use std::path::Path;

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
pub fn player(db: &Connection, id: u32) -> Result<Value> {
    use rusqlite::OptionalExtension;
    let raw: Option<String> = db
        .query_row("SELECT detail FROM players WHERE id=?", [id], |r| r.get(0))
        .optional()?;
    let raw = raw.ok_or_else(|| Error::new("NOT_FOUND", "Player not found."))?;
    Ok(serde_json::from_str(&raw)?)
}
