//! Helpers every record scan shares. The save has no index for its records, so each scan
//! slides over the database one byte at a time and asks whether a record starts there.
use super::archive::check;
use crate::{utils::cancel::CancelFlag, Result};

/// Slides over `from..to`. `read(offset)` says whether a record starts at `offset` and where
/// the scan resumes after it; a miss moves on one byte.
pub(super) fn scan<T>(
    from: usize,
    to: usize,
    cancel: &CancelFlag,
    mut read: impl FnMut(usize) -> Option<(T, usize)>,
) -> Result<Vec<T>> {
    let mut found = vec![];
    let mut offset = from;
    while offset < to {
        checkpoint(offset, cancel)?;
        match read(offset) {
            Some((record, next)) => {
                found.push(record);
                offset = next;
            }
            None => offset += 1,
        }
    }
    Ok(found)
}

/// Checks for cancellation every 64 KiB of a scan.
pub(super) fn checkpoint(offset: usize, cancel: &CancelFlag) -> Result<()> {
    if offset & 0xffff == 0 {
        check(cancel)?;
    }
    Ok(())
}

/// UTF-8 text without control characters or replacement characters.
pub(super) fn text(bytes: &[u8]) -> Option<String> {
    let s = std::str::from_utf8(bytes).ok()?;
    if s.chars().any(|c| c <= '\u{1f}' || c == '\u{fffd}') {
        None
    } else {
        Some(s.to_string())
    }
}

/// 0 and all-ones mark an unset id.
pub(super) fn is_set(id: u32) -> bool {
    id != 0 && id != u32::MAX
}
