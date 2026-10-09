use crate::{
    application::use_cases::{ports::SaveFiles, Session},
    domain::entities::{Job, Settings},
    Error, Result,
};
use std::sync::Arc;

pub struct SettingsUseCase {
    session: Arc<Session>,
    files: Arc<dyn SaveFiles>,
}
impl SettingsUseCase {
    pub fn new(session: Arc<Session>, files: Arc<dyn SaveFiles>) -> Self {
        Self { session, files }
    }
    /// The settings plus the import that is running now, if any.
    pub fn settings(&self) -> (Settings, Option<Job>) {
        let inner = self.session.inner.lock().unwrap();
        let job = inner
            .active
            .as_ref()
            .and_then(|a| inner.jobs.get(&a.id))
            .cloned();
        (inner.settings.clone(), job)
    }
    pub fn set_folder(&self, folder: Option<&str>) -> Result<Settings> {
        let mut inner = self.session.inner.lock().unwrap();
        if inner.active.is_some() {
            return Err(Error::query(
                "Wait for the current import before changing folders.",
            ));
        }
        let folder = folder
            .filter(|s| s.len() <= 4096)
            .ok_or_else(|| Error::query("Choose an existing save folder."))?;
        let settings = Settings {
            folder: self.files.directory(folder)?,
            last_snapshot: None,
        };
        self.session.persist(&settings)?;
        inner.settings = settings.clone();
        Ok(settings)
    }
}
