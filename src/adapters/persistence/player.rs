use super::{
    nation_name,
    snapshot::{open_snapshot, snapshot_file},
};
use crate::{
    application::use_cases::ports::PlayerRepository, domain::entities::SnapshotId, parser::CATALOG,
    utils::text::normalize, Error, Result,
};
use rusqlite::{params_from_iter, types::Value as SqlValue, Connection};
use serde_json::{json, Value};
use std::{collections::HashMap, path::PathBuf};

pub fn player(db: &Connection, id: u32) -> Result<Value> {
    use rusqlite::OptionalExtension;
    let raw: Option<String> = db
        .query_row("SELECT detail FROM players WHERE id=?", [id], |r| r.get(0))
        .optional()?;
    let raw = raw.ok_or_else(|| Error::new("NOT_FOUND", "Player not found."))?;
    Ok(serde_json::from_str(&raw)?)
}
pub fn search(db: &Connection, query: &str) -> Result<Value> {
    let mut params = HashMap::new();
    for (k, v) in url::form_urlencoded::parse(query.as_bytes()) {
        params.entry(k.into_owned()).or_insert(v.into_owned());
    }
    let integer = |key: &str, min: i64, max: i64| -> Result<Option<i64>> {
        let Some(s) = params.get(key).filter(|s| !s.is_empty()) else {
            return Ok(None);
        };
        let n = if s.trim().is_empty() {
            0.0
        } else {
            s.trim()
                .parse::<f64>()
                .map_err(|_| Error::query(format!("Invalid {key}.")))?
        };
        if !n.is_finite() || n.fract() != 0.0 || n < min as f64 || n > max as f64 {
            return Err(Error::query(format!("Invalid {key}.")));
        }
        Ok(Some(n as i64))
    };
    let mut clauses = vec![];
    let mut args: Vec<SqlValue> = vec![];
    if let Some(q) = params.get("q").map(|s| s.trim()).filter(|s| !s.is_empty()) {
        if q.encode_utf16().count() > 200 {
            return Err(Error::query("Search text is too long."));
        }
        clauses.push("search_name LIKE ? ESCAPE '\\'".into());
        args.push(
            format!(
                "%{}%",
                normalize(q)
                    .replace('\\', "\\\\")
                    .replace('%', "\\%")
                    .replace('_', "\\_")
            )
            .into(),
        );
    }
    for field in ["age", "ca", "pa"] {
        // Match JavaScript's exact integer range, without imposing slider limits.
        let min = integer(&format!("{field}Min"), 0, 9_007_199_254_740_991)?;
        let high = integer(&format!("{field}Max"), 0, 9_007_199_254_740_991)?;
        if min.zip(high).is_some_and(|(a, b)| a > b) {
            return Err(Error::query(format!("{field} minimum exceeds maximum.")));
        }
        if let Some(n) = min {
            clauses.push(format!("{field} >= ?"));
            args.push(n.into());
        }
        if let Some(n) = high {
            clauses.push(format!("{field} <= ?"));
            args.push(n.into());
        }
    }
    if let Some(n) = integer("valueMax", 0, 9_007_199_254_740_991)? {
        // unknown values stay in only when asked, e.g. free agents have none
        if params.get("valueUnknown").is_some_and(|v| v == "1") {
            clauses.push("(value <= ? OR value IS NULL)".into());
        } else {
            clauses.push("value <= ?".into());
        }
        args.push(n.into());
    }
    if let Some(list) = params.get("mainPosition").filter(|s| !s.is_empty()) {
        let mut marks = vec![];
        for id in list.split(',') {
            let i = id
                .trim()
                .parse::<usize>()
                .ok()
                .filter(|i| *i < CATALOG.positions.len())
                .ok_or_else(|| Error::query("Invalid mainPosition."))?;
            marks.push("?");
            args.push((i as i64).into());
        }
        clauses.push(format!("main_position IN ({})", marks.join(",")));
    }
    if let Some(list) = params.get("excludeIds").filter(|s| !s.is_empty()) {
        let ids = list
            .split(',')
            .map(|id| {
                id.trim()
                    .parse::<u32>()
                    .map_err(|_| Error::query("Invalid excludeIds."))
            })
            .collect::<Result<Vec<_>>>()?;
        if ids.len() > 5000 {
            return Err(Error::query("Too many excludeIds."));
        }
        clauses.push(format!("id NOT IN ({})", vec!["?"; ids.len()].join(",")));
        args.extend(ids.into_iter().map(|id| SqlValue::from(id as i64)));
    }
    if let Some(d) = params.get("contractEndMax").filter(|s| !s.is_empty()) {
        let b = d.as_bytes();
        let ok = b.len() == 10
            && b.iter().enumerate().all(|(i, c)| {
                if i == 4 || i == 7 {
                    *c == b'-'
                } else {
                    c.is_ascii_digit()
                }
            });
        if !ok {
            return Err(Error::query("Invalid contractEndMax. Expected YYYY-MM-DD."));
        }
        // ISO dates compare as text; NULL contracts never match
        clauses.push("contract_end <= ?".into());
        args.push(d.clone().into());
    }
    if params.get("noContract").is_some_and(|v| v == "1") {
        clauses.push("contract_end IS NULL".into());
    }
    if let Some(n) = integer("club", -1, 100000)? {
        if n == -1 {
            clauses.push("club_id IS NULL".into())
        } else {
            clauses.push("club_id = ?".into());
            args.push(n.into());
        }
    }
    if let Some(n) = integer("nation", 0, 255)? {
        clauses.push("EXISTS (SELECT 1 FROM json_each(players.nations) WHERE value = ?)".into());
        args.push(n.into());
    }
    let position_match = params
        .get("positionMatch")
        .map(String::as_str)
        .unwrap_or("and");
    if !["and", "or"].contains(&position_match) {
        return Err(Error::query(
            "Invalid positionMatch. Expected 'and' or 'or'.",
        ));
    }
    if let Some(positions) = params.get("position").filter(|s| !s.is_empty()) {
        let mut mask = 0i64;
        for id in positions.split(',') {
            let id = id.trim();
            if id.is_empty() || !id.bytes().all(|b| b.is_ascii_digit()) {
                return Err(Error::query("Invalid position."));
            }
            let index = id
                .parse::<usize>()
                .map_err(|_| Error::query("Invalid position."))?;
            if index >= CATALOG.positions.len() {
                return Err(Error::query("Invalid position."));
            }
            mask |= 1i64 << index;
        }
        if position_match == "and" {
            clauses.push("(position_mask & ?) = ?".into());
            args.push(mask.into());
            args.push(mask.into());
        } else {
            clauses.push("(position_mask & ?) != 0".into());
            args.push(mask.into());
        }
    }
    for a in &CATALOG.attributes {
        if let Some(n) = integer(&format!("attr_{}", a.key), 1, 20)? {
            clauses.push(format!("attr_{} >= ?", a.key));
            args.push(n.into());
        }
    }
    let sort = params
        .get("sort")
        .filter(|s| !s.is_empty())
        .map(String::as_str)
        .unwrap_or("pa");
    let column = match sort {
        "name" => "name COLLATE NOCASE".into(),
        "club" => "club COLLATE NOCASE".into(),
        "age" | "ca" | "pa" => sort.to_string(),
        key if CATALOG.attributes.iter().any(|a| a.key == key) => format!("attr_{key}"),
        _ => return Err(Error::query("Unsupported sort column.")),
    };
    let direction = params
        .get("direction")
        .filter(|s| !s.is_empty())
        .map(String::as_str)
        .unwrap_or("desc");
    if !["asc", "desc"].contains(&direction) {
        return Err(Error::query("Invalid sort direction."));
    }
    let page = integer("page", 1, 100000)?.unwrap_or(1);
    let limit = integer("limit", 1, 250)?.unwrap_or(100);
    let condition = if clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", clauses.join(" AND "))
    };
    let total: i64 = db.query_row(
        &format!("SELECT COUNT(*) FROM players {condition}"),
        params_from_iter(&args),
        |r| r.get(0),
    )?;
    args.extend([limit.into(), ((page - 1) * limit).into()]);
    let mut stmt = db.prepare(&format!("SELECT id,uid,name,age,club_id,club,nations,positions,ca,pa,wage,contract_end,value FROM players {condition} ORDER BY {column} {direction}, ca DESC, id ASC LIMIT ? OFFSET ?"))?;
    let players = stmt
        .query_map(params_from_iter(&args), |r| {
            let nations: String = r.get(6)?;
            let positions: String = r.get(7)?;
            Ok(json!({
                "id": r.get::<_, u32>(0)?, "uid": r.get::<_, u32>(1)?, "name": r.get::<_, String>(2)?,
                "age": r.get::<_, i32>(3)?, "club_id": r.get::<_, Option<u32>>(4)?, "club": r.get::<_, Option<String>>(5)?,
                "nationalities": serde_json::from_str::<Vec<u32>>(&nations).unwrap_or_default().into_iter().map(nation_name).collect::<Vec<_>>(),
                "positions": serde_json::from_str::<Value>(&positions).unwrap_or(Value::Null),
                "ca": r.get::<_, u8>(8)?, "pa": r.get::<_, u8>(9)?, "wage": r.get::<_, Option<u32>>(10)?,
                "contractEnd": r.get::<_, Option<String>>(11)?, "value": r.get::<_, Option<u32>>(12)?,
            }))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(json!({"total":total,"page":page,"limit":limit,"players":players}))
}

/// Player queries on the snapshot files in `dir`.
pub struct SqlitePlayerRepository {
    dir: PathBuf,
}
impl SqlitePlayerRepository {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }
}

impl PlayerRepository for SqlitePlayerRepository {
    fn player(&self, snapshot: &SnapshotId, id: u32) -> Result<Value> {
        player(&open_snapshot(&snapshot_file(&self.dir, snapshot)?)?, id)
    }
    fn search(&self, snapshot: &SnapshotId, query: &str) -> Result<Value> {
        search(&open_snapshot(&snapshot_file(&self.dir, snapshot)?)?, query)
    }
}
