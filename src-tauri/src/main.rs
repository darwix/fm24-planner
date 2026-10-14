#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use fm_squad_planner::infra::{
    app::{create_app, serve_web},
    config::default_data_dir,
    setup::init_app_state,
};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_dialog::DialogExt;

/// Runs the API server in-process on a free port and opens a window on it.
fn main() {
    let runtime = std::sync::Arc::new(tokio::runtime::Runtime::new().expect("async runtime"));
    let executor = runtime.clone();
    let (shutdown, stop) = tokio::sync::oneshot::channel::<()>();
    let mut shutdown = Some(shutdown);
    let mut stop = Some(stop);
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            let web = app.path().resource_dir()?.join("web");
            let started = executor.block_on(async {
                let (state, data) = init_app_state(&default_data_dir())?;
                let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
                let port = listener.local_addr()?.port();
                let router = serve_web(create_app(state, port), web);
                let stop = stop.take().expect("setup runs once");
                tokio::spawn(async move {
                    // `data` holds the directory lock until the server stops.
                    let _data = data;
                    axum::serve(listener, router)
                        .with_graceful_shutdown(async { stop.await.ok(); })
                        .await
                });
                Ok::<_, fm_squad_planner::Error>(port)
            });
            let port = match started {
                Ok(port) => port,
                Err(e) => {
                    app.dialog()
                        .message(e.message.clone())
                        .title("FM Squad Planner could not start")
                        .kind(tauri_plugin_dialog::MessageDialogKind::Error)
                        .blocking_show();
                    return Err(e.message.into());
                }
            };
            let url = format!("http://127.0.0.1:{port}");
            WebviewWindowBuilder::new(app, "main", WebviewUrl::External(url.parse()?))
                .title("FM Squad Planner")
                .inner_size(1440.0, 940.0)
                .min_inner_size(900.0, 640.0)
                .build()?;
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("tauri app")
        .run(move |_, event| {
            if let tauri::RunEvent::Exit = event {
                // ponytail: no wait for a running import; it dies with the process
                if let Some(tx) = shutdown.take() {
                    let _ = tx.send(());
                }
            }
        });
}
