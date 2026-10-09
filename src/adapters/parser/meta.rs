//! Facts about the save itself, outside the records: format version, in-game date and the
//! club the user manages.
use super::{archive::Archive, bytes};
use crate::{
    domain::entities::{Club, Player},
    utils::cancel::CancelFlag,
    Error, Result,
};
use chrono::{Duration, NaiveDate};
use std::{collections::HashSet, path::Path};

const SUPPORTED_VERSION: &str = "24.3.0+0";

/// Opens the archive and checks `game_info.dat` names an FM24 24.3 save.
pub fn open_supported(path: &Path, cancel: &CancelFlag) -> Result<(Archive, String)> {
    let mut archive = Archive::open(path, cancel)?;
    let info = archive.member("game_info.dat", cancel)?;
    let version = info
        .get(12..20)
        .and_then(|s| std::str::from_utf8(s).ok())
        .unwrap_or("unknown");
    if version != SUPPORTED_VERSION {
        return Err(Error::new(
            "UNSUPPORTED_VERSION",
            format!("Save format {version} is not supported. FM24 24.3 saves are required."),
        ));
    }
    Ok((archive, version.to_string()))
}

/// The in-game year and date, `[day u16][year u16]` at byte 36 of the database.
pub fn game_date(db: &[u8]) -> Result<(u16, String)> {
    if db.len() < 40 {
        return Err(Error::new(
            "TRUNCATED",
            "The player database header is incomplete.",
        ));
    }
    let (day, year) = (bytes::u16(db, 36) & 511, bytes::u16(db, 38));
    if !(1..=366).contains(&day) || !(2020..=2300).contains(&year) {
        return Err(Error::new(
            "UNSUPPORTED_DATE",
            "Could not read the in-game date.",
        ));
    }
    Ok((year, date_of(year, day)?))
}

/// `YYYY-MM-DD` for a one-based day of the year.
pub fn date_of(year: u16, day: u16) -> Result<String> {
    // Match JavaScript's one-based day-of-year rollover for day 366.
    let date = NaiveDate::from_ymd_opt(year as i32, 1, 1)
        .and_then(|d| d.checked_add_signed(Duration::days(day as i64 - 1)))
        .ok_or_else(|| Error::new("UNSUPPORTED_DATE", "Invalid in-game date."))?;
    Ok(date.format("%Y-%m-%d").to_string())
}

/// The club the user manages, named in the save summary. Only clubs with players count:
/// league names in the summary can collide with club-less records.
pub fn managed_club(
    archive: &mut Archive,
    clubs: &[Club],
    players: &[Player],
    cancel: &CancelFlag,
) -> Option<u32> {
    let with_players: HashSet<_> = players.iter().filter_map(|p| p.club_id).collect();
    let names: Vec<(u32, &str)> = clubs
        .iter()
        .filter(|c| with_players.contains(&c.id))
        .flat_map(|c| [(c.id, c.name.as_str()), (c.id, c.short_name.as_str())])
        .collect();
    let summary = archive.member("save_game_summary.dat", cancel).ok()?;
    managed_club_id(&summary, &names)
}

/// Managed club: first length-prefixed string in the save summary that names exactly one club.
pub fn managed_club_id(summary: &[u8], clubs: &[(u32, &str)]) -> Option<u32> {
    (0..summary.len().saturating_sub(4)).find_map(|i| {
        let n = u32::from_le_bytes(summary[i..i + 4].try_into().ok()?) as usize;
        let s = std::str::from_utf8(summary.get(i + 4..i + 4 + n)?).ok()?;
        let mut ids = clubs.iter().filter(|c| c.1 == s).map(|c| c.0);
        let id = ids.next()?;
        ids.all(|x| x == id).then_some(id)
    })
}
