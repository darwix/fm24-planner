use crate::{Error, Result};
use std::fmt;

/// Identifies an imported snapshot: a 64-character lowercase hex digest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnapshotId(String);

impl SnapshotId {
    pub fn parse(id: &str) -> Result<Self> {
        let valid = id.len() == 64
            && id
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c));
        if !valid {
            return Err(Error::query("Invalid snapshot."));
        }
        Ok(Self(id.to_owned()))
    }
}
impl fmt::Display for SnapshotId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
