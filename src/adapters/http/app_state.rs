use crate::application::use_cases::{PlayerUseCase, SaveUseCase, SettingsUseCase};
use std::sync::Arc;

/// What every route handler can reach: the use cases.
pub struct AppState {
    pub saves: Arc<SaveUseCase>,
    pub players: Arc<PlayerUseCase>,
    pub settings: Arc<SettingsUseCase>,
}
pub type Shared = Arc<AppState>;

impl AppState {
    pub fn new(
        saves: Arc<SaveUseCase>,
        players: Arc<PlayerUseCase>,
        settings: Arc<SettingsUseCase>,
    ) -> Shared {
        Arc::new(Self {
            saves,
            players,
            settings,
        })
    }
}
