use crate::{
    parser::{archive::check, ParsedSave, CATALOG, PARSER_VERSION},
    Error, Result,
};
use chrono::Utc;
use rusqlite::{params_from_iter, types::Value as SqlValue, Connection, OpenFlags};
use serde_json::{json, Value};
use std::{
    collections::{BTreeSet, HashMap},
    path::Path,
    sync::LazyLock,
};
use tokio_util::sync::CancellationToken;
use crate::utils::text::normalize;

static COUNTRIES: LazyLock<HashMap<String, String>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../../assets/nations.json")).expect("country catalog")
});
pub fn nation_name(id: u32) -> String {
    COUNTRIES
        .get(&id.to_string())
        .cloned()
        .unwrap_or_else(|| format!("Nation {id}"))
}

pub fn create_snapshot(
    path: &Path,
    save: &ParsedSave,
    mut meta: Value,
    cancel: &CancellationToken,
) -> Result<()> {
    let mut db = Connection::open(path)?;
    let attributes = CATALOG
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

