use crate::{application::session::SettingsRepository, domain::entities::Settings, Error, Result};
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
        Ok(Settings {
            folder: self.default_folder.clone(),
            last_snapshot: None,
            session: None,
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
