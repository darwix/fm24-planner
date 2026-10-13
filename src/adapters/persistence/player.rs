use super::snapshot::{open_snapshot, snapshot_file};
use crate::{
    application::use_cases::players::PlayerRepository,
    domain::entities::{
        ClubFilter, PlayerDetail, PlayerFilter, PlayerPage, PlayerSummary, SnapshotId, SortKey,
    },
    Result,
};
use rusqlite::{params_from_iter, types::Value as SqlValue, Connection, OptionalExtension};
use std::path::PathBuf;

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
    fn player(&self, snapshot: &SnapshotId, id: u32) -> Result<Option<PlayerDetail>> {
        player(&open_snapshot(&snapshot_file(&self.dir, snapshot)?)?, id)
    }
    fn search(&self, snapshot: &SnapshotId, filter: &PlayerFilter) -> Result<PlayerPage> {
        search(
            &open_snapshot(&snapshot_file(&self.dir, snapshot)?)?,
            filter,
        )
    }
}

fn player(db: &Connection, id: u32) -> Result<Option<PlayerDetail>> {
    let raw: Option<String> = db
        .query_row("SELECT detail FROM players WHERE id=?", [id], |r| r.get(0))
        .optional()?;
    raw.map(|raw| Ok(serde_json::from_str(&raw)?)).transpose()
}

fn escape_like(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn search(db: &Connection, filter: &PlayerFilter) -> Result<PlayerPage> {
    let mut clauses: Vec<String> = vec![];
    let mut args: Vec<SqlValue> = vec![];
    if let Some(text) = &filter.text {
        clauses.push("search_name LIKE ? ESCAPE '\\'".into());
        args.push(format!("%{}%", escape_like(text)).into());
    }
    for (column, range) in [("age", filter.age), ("ca", filter.ca), ("pa", filter.pa)] {
        if let Some(n) = range.min {
            clauses.push(format!("{column} >= ?"));
            args.push(n.into());
        }
        if let Some(n) = range.max {
            clauses.push(format!("{column} <= ?"));
            args.push(n.into());
        }
    }
    if let Some(n) = filter.value_max {
        clauses.push(
            if filter.value_unknown {
                "(value <= ? OR value IS NULL)"
            } else {
                "value <= ?"
            }
            .into(),
        );
        args.push(n.into());
    }
    // `column IN (?,?,..)` (or NOT IN) with one argument per value; nothing when empty.
    let mut in_list = |condition: &str, values: Vec<i64>| {
        if !values.is_empty() {
            clauses.push(format!(
                "{condition} ({})",
                vec!["?"; values.len()].join(",")
            ));
            args.extend(values.into_iter().map(SqlValue::from));
        }
    };
    in_list(
        "main_position IN",
        filter.main_positions.iter().map(|&i| i as i64).collect(),
    );
    in_list(
        "id NOT IN",
        filter.exclude_ids.iter().map(|&id| id as i64).collect(),
    );
    if let Some(date) = &filter.contract_end_max {
        clauses.push(
            if filter.or_no_contract {
                "(contract_end <= ? OR (contract_end IS NULL AND wage IS NULL))"
            } else {
                "contract_end <= ?"
            }
            .into(),
        );
        args.push(date.clone().into());
    }
    if filter.transfer_listed {
        clauses.push("transfer_listed = 1 AND coalesce(club_id, -1) != coalesce((SELECT json_extract(value, '$.managedClubId') FROM metadata WHERE key = 'snapshot'), -2)".into());
    }
    if filter.loan_listed {
        // like the game's search, leave out the managed club's own players
        clauses.push("loan_listed = 1 AND coalesce(club_id, -1) != coalesce((SELECT json_extract(value, '$.managedClubId') FROM metadata WHERE key = 'snapshot'), -2)".into());
    }
    if filter.shortlist {
        // the table is created on first shortlist use; before that nobody is on it
        let exists: bool = db.query_row(
            "SELECT EXISTS (SELECT 1 FROM sqlite_master WHERE type='table' AND name='shortlist')",
            [],
            |r| r.get(0),
        )?;
        clauses.push(
            if exists {
                "id IN (SELECT player_id FROM shortlist)"
            } else {
                "0"
            }
            .into(),
        );
    }
    if let Some(club) = filter.exclude_club {
        clauses.push("coalesce(club_id, -1) != ?".into());
        args.push((club as i64).into());
    }
    if filter.no_contract {
        // a wage without a decoded end date is still a contract
        clauses.push("contract_end IS NULL AND wage IS NULL".into());
    }
    match filter.club {
        Some(ClubFilter::Free) => clauses.push("club_id IS NULL".into()),
        Some(ClubFilter::Id(id)) => {
            clauses.push("club_id = ?".into());
            args.push((id as i64).into());
        }
        None => {}
    }
    if let Some(n) = filter.nation {
        clauses.push("EXISTS (SELECT 1 FROM json_each(players.nations) WHERE value = ?)".into());
        args.push(n.into());
    }
    if let Some(positions) = &filter.positions {
        if positions.all {
            clauses.push("(position_mask & ?) = ?".into());
            args.push(positions.mask.into());
        } else {
            clauses.push("(position_mask & ?) != 0".into());
        }
        args.push(positions.mask.into());
    }
    for (key, minimum) in &filter.attributes {
        clauses.push(format!("attr_{key} >= ?"));
        args.push((*minimum).into());
    }
    let column = match &filter.sort {
        SortKey::Name => "name COLLATE NOCASE".to_owned(),
        SortKey::Club => "club COLLATE NOCASE".to_owned(),
        SortKey::Age => "age".to_owned(),
        SortKey::Ca => "ca".to_owned(),
        SortKey::Pa => "pa".to_owned(),
        SortKey::Position => "main_position".to_owned(),
        SortKey::Value => "value".to_owned(),
        SortKey::Wage => "wage".to_owned(),
        SortKey::ContractEnd => "contract_end".to_owned(),
        SortKey::Attribute(key) => format!("attr_{key}"),
    };
    let direction = if filter.descending { "desc" } else { "asc" };
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
    args.extend([
        filter.limit.into(),
        ((filter.page - 1) * filter.limit).into(),
    ]);
    let mut stmt = db.prepare(&format!("SELECT id,uid,name,age,club_id,club,nationalities,positions,ca,pa,wage,contract_end,value,transfer_listed,loan_listed FROM players {condition} ORDER BY {column} IS NULL, {column} {direction}, ca DESC, id ASC LIMIT ? OFFSET ?"))?;
    let players = stmt
        .query_map(params_from_iter(&args), |r| {
            let nationalities: String = r.get(6)?;
            let positions: String = r.get(7)?;
            Ok(PlayerSummary {
                id: r.get(0)?,
                uid: r.get(1)?,
                name: r.get(2)?,
                age: r.get(3)?,
                club_id: r.get(4)?,
                club: r.get(5)?,
                nationalities: serde_json::from_str(&nationalities).unwrap_or_default(),
                positions: serde_json::from_str(&positions).unwrap_or_default(),
                ca: r.get(8)?,
                pa: r.get(9)?,
                wage: r.get(10)?,
                contract_end: r.get(11)?,
                value: r.get(12)?,
                transfer_listed: r.get(13)?,
                loan_listed: r.get(14)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(PlayerPage {
        total,
        page: filter.page,
        limit: filter.limit,
        players,
    })
}
