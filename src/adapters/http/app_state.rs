use crate::application::{
    use_cases::{PlayerUseCase, SaveUseCase, SettingsUseCase},
    Workspace,
};
use std::sync::Arc;

/// What every route handler can reach: the use cases.
pub struct AppState {
    pub saves: Arc<SaveUseCase>,
    pub players: Arc<PlayerUseCase>,
    pub settings: Arc<SettingsUseCase>,
}
pub type Shared = Arc<AppState>;

impl AppState {
    pub fn new(ws: Arc<Workspace>) -> Shared {
        Arc::new(Self {
            saves: Arc::new(SaveUseCase::new(ws.clone())),
            players: Arc::new(PlayerUseCase::new(ws.clone())),
            settings: Arc::new(SettingsUseCase::new(ws)),
        })
    }
}
