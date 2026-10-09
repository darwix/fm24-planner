use fm_savelens_backend::{
    application::use_cases::wait_for,
    infra::{
        app::create_app,
        config::{AppConfig, Command},
        setup::init_app_state,
    },
    Error, Result, VERSION,
};
use serde_json::json;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("{}: {}", e.code, e.message);
        std::process::exit(1);
    }
}

/// Parse arguments, serve, stop on Ctrl+C.
async fn run() -> Result<()> {
    let config = match AppConfig::from_args(std::env::args().skip(1))? {
        Command::Print(text) => {
            println!("{text}");
            return Ok(());
        }
        Command::Run(config) => config,
    };
    // `_data` holds the directory lock until the server has stopped.
    let (app_state, _data) = init_app_state(&config.data_dir)?;
    let listener = TcpListener::bind(("127.0.0.1", config.port)).await?;
    let address = listener.local_addr()?;
    let url = format!("http://{address}");
    let router = create_app(app_state.clone(), address.port());
    let shutdown = CancellationToken::new();
    let cancel = shutdown.clone();
    let server = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(cancel.cancelled_owned())
            .await
    });
    println!("FM SaveLens 24: {url}");
    println!(
        "{}",
        json!({"ready":true,"port":address.port(),"version":VERSION})
    );
    if !config.no_open {
        if let Err(e) = open::that_detached(&url) {
            eprintln!("Open {url} in your browser: {e}");
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
    let completion = app_state.saves.begin_shutdown();
    shutdown.cancel();
    if let Some(completion) = completion {
        tokio::task::spawn_blocking(move || wait_for(&completion))
            .await
            .map_err(|e| Error::new("SHUTDOWN", e.to_string()))?;
    }
    server
        .await
        .map_err(|e| Error::new("SHUTDOWN", e.to_string()))??;
    Ok(())
}
