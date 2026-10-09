use crate::{
    application::use_cases::ports::{SaveReader, SnapshotStore},
    domain::entities::{Job, Settings},
    utils::files::canonical,
    Error, Result,
};
use fs2::FileExt;
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Condvar, Mutex},
};
use tokio_util::sync::CancellationToken;

pub type Completion = Arc<(Mutex<bool>, Condvar)>;
#[derive(Clone)]
pub(crate) struct Active {
    pub id: String,
    pub cancel: CancellationToken,
    pub done: Completion,
}
pub(crate) struct Inner {
    pub settings: Settings,
    pub jobs: BTreeMap<String, Job>,
    pub active: Option<Active>,
    pub shutting_down: bool,
}
/// Runtime state shared by the use cases: settings, import jobs and the two ports.
pub struct Workspace {
    pub(crate) data: PathBuf,
    pub(crate) inner: Mutex<Inner>,
    pub(crate) reader: Arc<dyn SaveReader>,
    pub(crate) store: Arc<dyn SnapshotStore>,
    _lock: File,
}
impl Workspace {
    pub fn open(
        data: &Path,
        reader: Arc<dyn SaveReader>,
        store: Arc<dyn SnapshotStore>,
        default_folder: String,
    ) -> Result<Arc<Self>> {
        fs::create_dir_all(data)?;
        let data = canonical(data)?;
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(data.join("app.lock"))?;
        lock.try_lock_exclusive().map_err(|_|Error::new("ALREADY_RUNNING","FM SaveLens 24 is already using this data directory. Close it before starting another instance."))?;
        fs::create_dir_all(data.join("snapshots"))?;
        for entry in fs::read_dir(data.join("snapshots"))? {
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
        let settings_path = data.join("settings.json");
        let settings = if settings_path.exists() {
            serde_json::from_slice(&fs::read(&settings_path)?)?
        } else {
            let legacy = std::env::var_os("LOCALAPPDATA")
                .map(PathBuf::from)
                .map(|p| p.join("FM-SaveLens-24/settings.json"));
            let folder = legacy
                .and_then(|p| fs::read(p).ok())
                .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
                .and_then(|v| v["folder"].as_str().map(str::to_owned))
                .unwrap_or(default_folder);
            Settings {
                folder,
                last_snapshot: None,
            }
        };
        let app = Arc::new(Self {
            data,
            inner: Mutex::new(Inner {
                settings,
                jobs: BTreeMap::new(),
                active: None,
                shutting_down: false,
            }),
            reader,
            store,
            _lock: lock,
        });
        app.persist(&app.inner.lock().unwrap().settings)?;
        Ok(app)
    }
    pub(crate) fn persist(&self, settings: &Settings) -> Result<()> {
        let mut file = tempfile::NamedTempFile::new_in(&self.data)?;
        file.write_all(&serde_json::to_vec_pretty(settings)?)?;
        file.as_file().sync_all()?;
        file.persist(self.data.join("settings.json"))
            .map_err(|e| Error::from(e.error))?;
        Ok(())
    }
}
