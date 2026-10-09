use crate::{Error, Result};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, Metadata},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

pub fn mtime(stat: &Metadata) -> Result<f64> {
    Ok(stat
        .modified()?
        .duration_since(UNIX_EPOCH)
        .map_err(|_| Error::new("IO", "Unsupported file timestamp."))?
        .as_secs_f64()
        * 1000.0)
}
pub fn digest(s: &str) -> String {
    format!("{:x}", Sha256::digest(s.as_bytes()))
}
pub fn canonical(path: &Path) -> Result<PathBuf> {
    let path = fs::canonicalize(path)?;
    // Keep ordinary drive paths in the API and migration settings on Windows.
    #[cfg(windows)]
    {
        let s = path.to_string_lossy();
        if let Some(p) = s.strip_prefix(r"\\?\UNC\") {
            return Ok(PathBuf::from(format!(r"\\{p}")));
        }
        if let Some(p) = s.strip_prefix(r"\\?\") {
            return Ok(PathBuf::from(p));
        }
    }
    Ok(path)
}
