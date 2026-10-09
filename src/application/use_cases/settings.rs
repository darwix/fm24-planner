use crate::{
    application::Workspace,
    domain::entities::{Job, Settings},
    utils::files::canonical,
    Error, Result,
};
use std::{path::Path, sync::Arc};

pub struct SettingsUseCase {
    ws: Arc<Workspace>,
}
impl SettingsUseCase {
    pub fn new(ws: Arc<Workspace>) -> Self {
        Self { ws }
    }
    /// The settings plus the import that is running now, if any.
    pub fn settings(&self) -> (Settings, Option<Job>) {
        let inner = self.ws.inner.lock().unwrap();
        let job = inner
            .active
            .as_ref()
            .and_then(|a| inner.jobs.get(&a.id))
            .cloned();
        (inner.settings.clone(), job)
    }
    pub fn set_folder(&self, folder: Option<&str>) -> Result<Settings> {
        let mut inner = self.ws.inner.lock().unwrap();
        if inner.active.is_some() {
            return Err(Error::query(
                "Wait for the current import before changing folders.",
            ));
        }
        let folder = folder
            .filter(|s| s.len() <= 4096 && Path::new(s).is_dir())
            .ok_or_else(|| Error::query("Choose an existing save folder."))?;
        let settings = Settings {
            folder: canonical(Path::new(folder))?.to_string_lossy().into_owned(),
            last_snapshot: None,
        };
        self.ws.persist(&settings)?;
        inner.settings = settings.clone();
        Ok(settings)
    }
}
