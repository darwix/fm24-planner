use crate::{
    application::use_cases::ports::SettingsRepository,
    domain::entities::{Job, Settings},
    Result,
};
use std::{
    collections::BTreeMap,
    sync::{Arc, Condvar, Mutex},
};
use tokio_util::sync::CancellationToken;

pub type Completion = Arc<(Mutex<bool>, Condvar)>;

#[derive(Clone)]
pub(super) struct Active {
    pub id: String,
    pub cancel: CancellationToken,
    pub done: Completion,
}
pub(super) struct Inner {
    pub settings: Settings,
    pub jobs: BTreeMap<String, Job>,
    pub active: Option<Active>,
    pub shutting_down: bool,
}

/// In-memory state the save and settings use cases share: the current
/// settings, the import jobs and whether the app is shutting down.
pub struct Session {
    pub(super) inner: Mutex<Inner>,
    store: Arc<dyn SettingsRepository>,
}
impl Session {
    pub fn new(store: Arc<dyn SettingsRepository>) -> Result<Arc<Self>> {
        let settings = store.load()?;
        store.save(&settings)?;
        Ok(Arc::new(Self {
            inner: Mutex::new(Inner {
                settings,
                jobs: BTreeMap::new(),
                active: None,
                shutting_down: false,
            }),
            store,
        }))
    }
    pub(super) fn persist(&self, settings: &Settings) -> Result<()> {
        self.store.save(settings)
    }
}
