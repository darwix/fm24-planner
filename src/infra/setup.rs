use crate::{
    adapters::{
        http::app_state::{AppState, Shared},
        parser::reader::FmSaveReader,
        persistence::{
            FsSaveFiles, JsonSettingsRepository, SqlitePlayerRepository, SqliteSnapshotRepository,
        },
    },
    application::use_cases::{wait_for, PlayerUseCase, SaveUseCase, Session, SettingsUseCase},
    infra::{
        app::create_app,
        config::{default_folder, AppConfig, Command},
        data_dir::DataDir,
    },
    Error, Result, VERSION,
};
use serde_json::json;
use std::{net::SocketAddr, path::Path, sync::Arc};
use tokio::{net::TcpListener, task::JoinHandle};
use tokio_util::sync::CancellationToken;

pub struct RunningServer {
    pub address: SocketAddr,
    pub app: Shared,
    shutdown: CancellationToken,
    task: JoinHandle<std::io::Result<()>>,
    _data: DataDir,
}

impl RunningServer {
    pub fn url(&self) -> String {
        format!("http://{}", self.address)
    }
    pub async fn shutdown(self) -> Result<()> {
        let completion = self.app.saves.begin_shutdown();
        self.shutdown.cancel();
        if let Some(completion) = completion {
            tokio::task::spawn_blocking(move || wait_for(&completion))
                .await
                .map_err(|e| Error::new("SHUTDOWN", e.to_string()))?;
        }
        self.task
            .await
            .map_err(|e| Error::new("SHUTDOWN", e.to_string()))??;
        Ok(())
    }
}

/// Opens the data directory, binds `port` (0 = any), and serves until `shutdown`.
pub async fn start(port: u16, data: &Path) -> Result<RunningServer> {
    let data = DataDir::open(data)?;
    let files = Arc::new(FsSaveFiles);
    let session = Session::new(Arc::new(JsonSettingsRepository::new(
        data.root.clone(),
        default_folder(),
    )))?;
    let app = AppState::new(
        Arc::new(SaveUseCase::new(
            session.clone(),
            Arc::new(FmSaveReader),
            files.clone(),
            Arc::new(SqliteSnapshotRepository::new(data.snapshots())),
        )),
        Arc::new(PlayerUseCase::new(Arc::new(SqlitePlayerRepository::new(
            data.snapshots(),
        )))),
        Arc::new(SettingsUseCase::new(session, files)),
    );
    let listener = TcpListener::bind(("127.0.0.1", port)).await?;
    let address = listener.local_addr()?;
    let router = create_app(app.clone(), address.port());
    let shutdown = CancellationToken::new();
    let cancel = shutdown.clone();
    let task = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(cancel.cancelled_owned())
            .await
    });
    Ok(RunningServer {
        address,
        app,
        shutdown,
        task,
        _data: data,
    })
}

/// The command-line entry point: parse arguments, serve, stop on Ctrl+C.
pub async fn run(args: impl Iterator<Item = String>) -> Result<()> {
    let config = match AppConfig::from_args(args)? {
        Command::Print(text) => {
            println!("{text}");
            return Ok(());
        }
        Command::Run(config) => config,
    };
    let server = start(config.port, &config.data_dir).await?;
    println!("FM SaveLens 24: {}", server.url());
    println!(
        "{}",
        json!({"ready":true,"port":server.address.port(),"version":VERSION})
    );
    if !config.no_open {
        if let Err(e) = open::that_detached(server.url()) {
            eprintln!("Open {} in your browser: {e}", server.url());
        }
    }
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
        tokio::select! {r=tokio::signal::ctrl_c()=>{r?},_=terminate.recv()=>{}}
    }
    #[cfg(not(unix))]
    tokio::signal::ctrl_c().await?;
    server.shutdown().await
}
