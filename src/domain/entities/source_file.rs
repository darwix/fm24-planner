use std::{path::PathBuf, time::SystemTime};

/// A file on disk as the import sees it: enough to tell whether it changed.
#[derive(Clone)]
pub struct SourceFile {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    /// Milliseconds since the Unix epoch.
    pub mtime: f64,
    pub modified: SystemTime,
}
