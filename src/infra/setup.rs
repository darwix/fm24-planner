use crate::{
    adapters::{
        http::app_state::{AppState, Shared},
        parser::reader::FmSaveReader,
        persistence::{
            FsSaveFiles, JsonSettingsRepository, SqlitePlannerRepository, SqlitePlayerRepository,
            SqliteSnapshotRepository,
        },
    },
    application::{
        session::Session,
        use_cases::{PlannerUseCase, PlayerUseCase, SaveUseCase, SettingsUseCase},
    },
    infra::{config::default_folder, data_dir::DataDir},
    Result,
};
use std::{path::Path, sync::Arc};

/// Opens the data directory and wires the adapters into the use cases.
/// The returned `DataDir` holds the directory lock, so keep it alive as long as the state.
pub fn init_app_state(data: &Path) -> Result<(Shared, DataDir)> {
    let data = DataDir::open(data)?;
    let files = Arc::new(FsSaveFiles);
    let session = Session::new(Arc::new(JsonSettingsRepository::new(
        data.root.clone(),
        default_folder(),
    )))?;
    let state = AppState::new(
        Arc::new(SaveUseCase::new(
            session.clone(),
            Arc::new(FmSaveReader),
            files.clone(),
            Arc::new(SqliteSnapshotRepository::new(data.snapshots())),
        )),
        Arc::new(PlayerUseCase::new(Arc::new(SqlitePlayerRepository::new(
            data.snapshots(),
        )))),
        Arc::new(PlannerUseCase::new(Arc::new(SqlitePlannerRepository::new(
            data.snapshots(),
        )))),
        Arc::new(SettingsUseCase::new(session, files)),
    );
    Ok((state, data))
}
