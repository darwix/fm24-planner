//! Turns a player search request (query string or JSON body) into a validated `PlayerFilter`.
use crate::{
    domain::text::normalize,
    domain::{
        catalog::CATALOG,
        entities::{ClubFilter, PlayerFilter, PositionFilter, Range, SortKey},
    },
    Error, Result,
};
use serde_json::Value;
use std::collections::HashMap;

/// JavaScript's largest exact integer, so any number the web app sends is accepted.
const MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;
const MAX_TEXT_UNITS: usize = 200;
const MAX_EXCLUDED: usize = 5000;
const MAX_CLUB: i64 = 100_000;
const MAX_NATION: i64 = 255;
const MAX_PAGE: i64 = 100_000;
const MAX_LIMIT: i64 = 250;

/// Reads the search query string into a validated filter.
pub fn parse_filter(query: &str) -> Result<PlayerFilter> {
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
    let flag = |key: &str| params.get(key).is_some_and(|v| v == "1");
    // `{field}Min` and `{field}Max`, min not above max.
    let range = |field: &str| -> Result<Range> {
        let range = Range {
            min: integer(&format!("{field}Min"), 0, MAX_SAFE_INTEGER)?,
            max: integer(&format!("{field}Max"), 0, MAX_SAFE_INTEGER)?,
        };
        if range.min.zip(range.max).is_some_and(|(a, b)| a > b) {
            return Err(Error::query(format!("{field} minimum exceeds maximum.")));
        }
        Ok(range)
    };
    // A comma list of position indexes; empty when the key is missing.
    let positions = |key: &str| -> Result<Vec<usize>> {
        let Some(list) = params.get(key).filter(|s| !s.is_empty()) else {
            return Ok(vec![]);
        };
        list.split(',')
            .map(|id| {
                let id = id.trim();
                id.parse::<usize>()
                    .ok()
                    .filter(|i| {
                        id.bytes().all(|b| b.is_ascii_digit()) && *i < CATALOG.positions.len()
                    })
                    .ok_or_else(|| Error::query(format!("Invalid {key}.")))
            })
            .collect()
    };
    let mut filter = PlayerFilter::default();
    if let Some(q) = params.get("q").map(|s| s.trim()).filter(|s| !s.is_empty()) {
        if q.encode_utf16().count() > MAX_TEXT_UNITS {
            return Err(Error::query("Search text is too long."));
        }
        filter.text = Some(normalize(q));
    }
    filter.age = range("age")?;
    filter.ca = range("ca")?;
    filter.pa = range("pa")?;
    filter.value_max = integer("valueMax", 0, MAX_SAFE_INTEGER)?;
    filter.value_unknown = flag("valueUnknown");
    filter.main_positions = positions("mainPosition")?;
    if let Some(list) = params.get("excludeIds").filter(|s| !s.is_empty()) {
        filter.exclude_ids = list
            .split(',')
            .map(|id| {
                id.trim()
                    .parse::<u32>()
                    .map_err(|_| Error::query("Invalid excludeIds."))
            })
            .collect::<Result<Vec<_>>>()?;
        if filter.exclude_ids.len() > MAX_EXCLUDED {
            return Err(Error::query("Too many excludeIds."));
        }
    }
    if let Some(date) = params.get("contractEndMax").filter(|s| !s.is_empty()) {
        if !is_iso_date(date) {
            return Err(Error::query("Invalid contractEndMax. Expected YYYY-MM-DD."));
        }
        filter.contract_end_max = Some(date.clone());
    }
    filter.no_contract = flag("noContract");
    filter.club = integer("club", -1, MAX_CLUB)?.map(|n| match n {
        -1 => ClubFilter::Free,
        id => ClubFilter::Id(id as u32),
    });
    filter.nation = integer("nation", 0, MAX_NATION)?;
    let position_match = params
        .get("positionMatch")
        .map(String::as_str)
        .unwrap_or("and");
    if !["and", "or"].contains(&position_match) {
        return Err(Error::query(
            "Invalid positionMatch. Expected 'and' or 'or'.",
        ));
    }
    let wanted = positions("position")?;
    if !wanted.is_empty() {
        filter.positions = Some(PositionFilter {
            mask: wanted.iter().fold(0i64, |mask, i| mask | 1 << i),
            all: position_match == "and",
        });
    }
    for a in &CATALOG.attributes {
        if let Some(n) = integer(&format!("attr_{}", a.key), 1, 20)? {
            filter.attributes.push((a.key.clone(), n));
        }
    }
    let sort = params
        .get("sort")
        .filter(|s| !s.is_empty())
        .map(String::as_str)
        .unwrap_or("pa");
    filter.sort = match sort {
        "name" => SortKey::Name,
        "club" => SortKey::Club,
        "age" => SortKey::Age,
        "ca" => SortKey::Ca,
        "pa" => SortKey::Pa,
        key if CATALOG.attributes.iter().any(|a| a.key == key) => {
            SortKey::Attribute(key.to_owned())
        }
        _ => return Err(Error::query("Unsupported sort column.")),
    };
    filter.descending = match params
        .get("direction")
        .filter(|s| !s.is_empty())
        .map(String::as_str)
        .unwrap_or("desc")
    {
        "desc" => true,
        "asc" => false,
        _ => return Err(Error::query("Invalid sort direction.")),
    };
    filter.page = integer("page", 1, MAX_PAGE)?.unwrap_or(filter.page);
    filter.limit = integer("limit", 1, MAX_LIMIT)?.unwrap_or(filter.limit);
    Ok(filter)
}

fn is_iso_date(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b.iter().enumerate().all(|(i, c)| {
            if i == 4 || i == 7 {
                *c == b'-'
            } else {
                c.is_ascii_digit()
            }
        })
}

/// Turns a JSON filter object into the query string the GET search takes.
/// Arrays become comma lists, booleans 1 or 0, nulls are skipped.
pub fn query_from_body(body: &Value) -> String {
    let mut out = url::form_urlencoded::Serializer::new(String::new());
    for (k, v) in body.as_object().into_iter().flatten() {
        let text = match v {
            Value::Null => continue,
            Value::Bool(b) => (*b as u8).to_string(),
            Value::String(s) => s.clone(),
            Value::Array(a) => a
                .iter()
                .map(|x| x.as_str().map_or_else(|| x.to_string(), str::to_string))
                .collect::<Vec<_>>()
                .join(","),
            other => other.to_string(),
        };
        out.append_pair(k, &text);
    }
    out.finish()
}
