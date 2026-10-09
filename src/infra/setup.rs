use crate::{
    infra::{
        app::create_app,
        config::{AppConfig, Command},
    },
    services::{wait_for, AppState},
    Error, Result, VERSION,
};
use serde_json::json;
use std::{net::SocketAddr, path::Path, sync::Arc};
use tokio::{net::TcpListener, task::JoinHandle};
use tokio_util::sync::CancellationToken;

pub struct RunningServer {
    pub address: SocketAddr,
    pub app: Arc<AppState>,
    shutdown: CancellationToken,
    task: JoinHandle<std::io::Result<()>>,
}

impl RunningServer {
    pub fn url(&self) -> String {
        format!("http://{}", self.address)
    }
    pub async fn shutdown(self) -> Result<()> {
        let completion = self.app.begin_shutdown();
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
    let app = AppState::open(data)?;
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
