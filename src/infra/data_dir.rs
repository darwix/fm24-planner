use crate::{utils::files::canonical, Error, Result};
use fs2::FileExt;
use std::{
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
};

/// The app's data directory, locked so only one instance uses it.
pub struct DataDir {
    pub root: PathBuf,
    _lock: File,
}
impl DataDir {
    pub fn open(path: &Path) -> Result<Self> {
        fs::create_dir_all(path)?;
        let root = canonical(path)?;
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(root.join("app.lock"))?;
        lock.try_lock_exclusive().map_err(|_| {
            Error::new(
                "ALREADY_RUNNING",
                "FM Squad Planner is already using this data directory. Close it before starting another instance.",
            )
        })?;
        let data = Self { root, _lock: lock };
        fs::create_dir_all(data.snapshots())?;
        // Imports that never finished leave partial files behind.
        for entry in fs::read_dir(data.snapshots())? {
            let entry = entry?;
            if entry.file_type()?.is_file()
                && entry
                    .file_name()
                    .to_string_lossy()
                    .ends_with(".partial.sqlite")
            {
                fs::remove_file(entry.path())?;
            }
        }
        Ok(data)
    }
    pub fn snapshots(&self) -> PathBuf {
        self.root.join("snapshots")
    }
}
