use crate::application::use_cases::{PlannerUseCase, PlayerUseCase, SaveUseCase, SettingsUseCase};
use std::sync::Arc;

/// What every route handler can reach: the use cases.
pub struct AppState {
    pub saves: Arc<SaveUseCase>,
    pub players: Arc<PlayerUseCase>,
    pub planner: Arc<PlannerUseCase>,
    pub settings: Arc<SettingsUseCase>,
}
pub type Shared = Arc<AppState>;

impl AppState {
    pub fn new(
        saves: Arc<SaveUseCase>,
        players: Arc<PlayerUseCase>,
        planner: Arc<PlannerUseCase>,
        settings: Arc<SettingsUseCase>,
    ) -> Shared {
        Arc::new(Self {
            saves,
            players,
            planner,
            settings,
        })
    }
}
