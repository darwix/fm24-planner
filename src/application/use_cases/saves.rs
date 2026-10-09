use crate::{
    application::use_cases::{
        ports::{SaveFiles, SaveReader, SnapshotRepository},
        Session,
    },
    domain::entities::{SaveFile, SnapshotId},
    utils::files::digest,
    Result,
};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use std::{path::Path, sync::Arc};

pub struct SaveUseCase {
    pub(super) session: Arc<Session>,
    pub(super) reader: Arc<dyn SaveReader>,
    pub(super) files: Arc<dyn SaveFiles>,
    pub(super) snapshots: Arc<dyn SnapshotRepository>,
}
impl SaveUseCase {
    pub fn new(
        session: Arc<Session>,
        reader: Arc<dyn SaveReader>,
        files: Arc<dyn SaveFiles>,
        snapshots: Arc<dyn SnapshotRepository>,
    ) -> Self {
        Self {
            session,
            reader,
            files,
            snapshots,
        }
    }
    pub fn parser_version(&self) -> &str {
        self.reader.version()
    }
    pub fn list_saves(&self) -> Result<Vec<SaveFile>> {
        let folder = self.session.inner.lock().unwrap().settings.folder.clone();
        self.list(&folder)
    }
    pub(super) fn list(&self, folder: &str) -> Result<Vec<SaveFile>> {
        if folder.is_empty() {
            return Ok(vec![]);
        }
        let mut saves = vec![];
        for file in self.files.scan(folder)? {
            let path = file.path.to_string_lossy().into_owned();
            let snapshot_id = digest(&format!(
                "{path}|{}|{}|{}",
                file.size,
                file.mtime,
                self.reader.version()
            ));
            saves.push(SaveFile {
                id: digest(&path),
                name: file.name,
                size: file.size,
                mtime: file.mtime,
                modified: DateTime::<Utc>::from(file.modified)
                    .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                cached: self.snapshots.exists(&SnapshotId::parse(&snapshot_id)?),
                snapshot_id,
            });
        }
        saves.sort_by(|a, b| b.modified.cmp(&a.modified));
        Ok(saves)
    }
    /// An imported save's metadata, flagged stale when the source file or the
    /// parser changed since the import.
    pub fn snapshot(&self, id: &str) -> Result<Value> {
        let mut meta = self.snapshots.metadata(&SnapshotId::parse(id)?)?;
        let stale = match self
            .files
            .stat(Path::new(meta["sourcePath"].as_str().unwrap_or("")))
        {
            Ok(file) => {
                file.size != meta["sourceSize"].as_u64().unwrap_or(0)
                    || Some(file.mtime) != meta["sourceMtime"].as_f64()
                    || meta["parserVersion"].as_str() != Some(self.reader.version())
            }
            Err(_) => true,
        };
        meta["stale"] = json!(stale);
        Ok(meta)
    }
}
