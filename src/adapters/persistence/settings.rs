use crate::{application::session::SettingsRepository, domain::entities::Settings, Error, Result};
use serde_json::Value;
use std::{fs, io::Write, path::PathBuf};

/// Settings kept in `settings.json` inside the data directory.
pub struct JsonSettingsRepository {
    dir: PathBuf,
    default_folder: String,
}
impl JsonSettingsRepository {
    pub fn new(dir: PathBuf, default_folder: String) -> Self {
        Self {
            dir,
            default_folder,
        }
    }
}

impl SettingsRepository for JsonSettingsRepository {
    fn load(&self) -> Result<Settings> {
        let path = self.dir.join("settings.json");
        if path.exists() {
            return Ok(serde_json::from_slice(&fs::read(path)?)?);
        }
        // Migrate the folder from the earlier desktop app's settings, if present.
        let legacy = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .map(|p| p.join("FM-SaveLens-24/settings.json"));
        let folder = legacy
            .and_then(|p| fs::read(p).ok())
            .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
            .and_then(|v| v["folder"].as_str().map(str::to_owned))
            .unwrap_or_else(|| self.default_folder.clone());
        Ok(Settings {
            folder,
            last_snapshot: None,
        })
    }
    fn save(&self, settings: &Settings) -> Result<()> {
        let mut file = tempfile::NamedTempFile::new_in(&self.dir)?;
        file.write_all(&serde_json::to_vec_pretty(settings)?)?;
        file.as_file().sync_all()?;
        file.persist(self.dir.join("settings.json"))
            .map_err(|e| Error::from(e.error))?;
        Ok(())
    }
}
