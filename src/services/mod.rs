use crate::{
    domain::entities::{Job, SaveFile, Settings},
    parser::{self, archive::check, PARSER_VERSION},
    storage,
    infra::config::default_folder,
    utils::files::{canonical, digest, mtime},
    Error, Result, VERSION,
};
use axum::{
    body::{to_bytes, Body},
    extract::State,
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use fs2::FileExt;
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Condvar, Mutex},
    time::Instant,
};
use tokio_util::sync::CancellationToken;

pub type Completion = Arc<(Mutex<bool>, Condvar)>;
#[derive(Clone)]
struct Active {
    id: String,
    cancel: CancellationToken,
    done: Completion,
}
struct Inner {
    settings: Settings,
    jobs: BTreeMap<String, Job>,
    active: Option<Active>,
    shutting_down: bool,
}
pub struct AppState {
    data: PathBuf,
    inner: Mutex<Inner>,
    _lock: File,
}
impl AppState {
    pub fn open(data: &Path) -> Result<Arc<Self>> {
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
                .unwrap_or_else(default_folder);
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
            _lock: lock,
        });
        app.persist(&app.inner.lock().unwrap().settings)?;
        Ok(app)
    }
    fn persist(&self, settings: &Settings) -> Result<()> {
        let mut file = tempfile::NamedTempFile::new_in(&self.data)?;
        file.write_all(&serde_json::to_vec_pretty(settings)?)?;
        file.as_file().sync_all()?;
        file.persist(self.data.join("settings.json"))
            .map_err(|e| Error::from(e.error))?;
        Ok(())
    }
}

mod import;
pub mod routes;
mod saves;
pub use import::wait_for;
pub use routes::{error_response, json_response};
