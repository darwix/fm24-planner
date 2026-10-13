use std::{fs, path::Path};

fn put32(b: &mut [u8], at: usize, n: u32) {
    b[at..at + 4].copy_from_slice(&n.to_le_bytes());
}
fn put16(b: &mut [u8], at: usize, n: u16) {
    b[at..at + 2].copy_from_slice(&n.to_le_bytes());
}
fn text(b: &mut Vec<u8>, s: &str) {
    b.extend_from_slice(&(s.len() as u32).to_le_bytes());
    b.extend_from_slice(s.as_bytes());
}
pub fn database(count: usize) -> Vec<u8> {
    let mut b = vec![0; 512];
    put16(&mut b, 36, 100);
    put16(&mut b, 38, 2024);
    // A small valid club table before the names; players intentionally have no verified team.
    put32(&mut b, 128, 300);
    put32(&mut b, 132, 1_000_000);
    put32(&mut b, 136, 1_000_000);
    put32(&mut b, 141, 10);
    b[145..149].fill(255);
    put32(&mut b, 153, 10);
    put32(&mut b, 167, 9);
    b[171..180].copy_from_slice(b"Test Club");
    put32(&mut b, 180, 4);
    b[184..188].copy_from_slice(b"Test");
    for (first, total) in [(true, 10000), (false, 10000), (false, 1)] {
        b.extend_from_slice(&(total as u32).to_le_bytes());
        for i in 0..total {
            b.extend_from_slice(&(i as u32).to_le_bytes());
            text(
                &mut b,
                if first && i == 0 {
                    "Álvaro"
                } else if !first && i == 0 {
                    "García"
                } else {
                    "Name"
                },
            );
        }
    }
    let start = b.len();
    b.resize(start + 256 + count * 512, 0);
    put32(&mut b, start, count as u32 + 10);
    for i in 0..count {
        let base = start + 256 + i * 512;
        let a = base + 57;
        let id = i as u32 + 1;
        let uid = 2_000_000_000 + i as u32;
        put32(&mut b, base, id - 1);
        put32(&mut b, base + 4, uid);
        put32(&mut b, base + 8, uid + 100);
        b[a - 38] = 200;
        b[a - 36] = 199;
        b[a - 15..a].fill(1);
        b[a - 3] = 20;
        b[a..a + 54].fill(63);
        let p = base + 160;
        put32(&mut b, p, 0);
        put32(&mut b, p + 5, 0);
        put32(&mut b, p + 10, u32::MAX);
        put32(&mut b, p + 15, 0);
        let end = p + 19;
        put16(&mut b, end, 60);
        put16(&mut b, end + 2, 2000);
        put16(&mut b, end + 9, 10);
        b[end + 17..end + 25].fill(10);
        b[end + 33] = 1;
        b[end + 34] = 1;
        put32(&mut b, end + 35, 12);
        b[end + 45] = 8;
        b[end + 46] = 70;
        let identity = end + 80;
        put16(&mut b, identity - 12, 100);
        put16(&mut b, identity - 10, 2024);
        b[identity - 6] = 0x20;
        put32(&mut b, identity, id);
        put32(&mut b, identity + 4, uid);
        put32(&mut b, identity + 8, uid + 100);
    }
    b
}
pub fn archive(path: &Path, compressed: bool, version: &str, count: usize) {
    let mut info = vec![0; 32];
    info[12..20].copy_from_slice(version.as_bytes());
    write_archive(
        path,
        compressed,
        vec![("game_info", info), ("game_db", database(count))],
    );
}
pub fn write_archive(path: &Path, compressed: bool, members: Vec<(&str, Vec<u8>)>) {
    let mut output = vec![0; 26];
    output[..9].copy_from_slice(b"\x02\x01fmf.\x08\0\0");
    output[25] = if compressed { 3 } else { 0 };
    let mut manifest = vec![];
    text(&mut manifest, "Synthetic career");
    manifest.extend_from_slice(&(members.len() as u32).to_le_bytes());
    for (name, plain) in members {
        let stored = if compressed {
            zstd::stream::encode_all(&plain[..], 1).unwrap()
        } else {
            plain.clone()
        };
        text(&mut manifest, name);
        text(&mut manifest, ".dat");
        manifest.extend_from_slice(&(output.len() as u64 - 26).to_le_bytes());
        manifest.extend_from_slice(&(stored.len() as u64).to_le_bytes());
        manifest.extend_from_slice(&(plain.len() as u64).to_le_bytes());
        manifest.extend_from_slice(&[0; 16]);
        output.extend(stored);
    }
    manifest.extend_from_slice(&0u32.to_le_bytes());
    let start = output.len();
    output[9..17].copy_from_slice(&(start as u64 - 9).to_le_bytes());
    output.extend_from_slice(b"\x02\x01fmf.\x08\0\0");
    output.extend(zstd::stream::encode_all(&manifest[..], 1).unwrap());
    fs::write(path, output).unwrap();
}

/// A snapshot written through the real repositories, for search tests.
#[allow(dead_code)]
pub struct TestSnapshot {
    /// The SQLite file, for tests that adjust rows directly.
    pub path: std::path::PathBuf,
    id: fm_savelens_backend::domain::entities::SnapshotId,
    players: fm_savelens_backend::adapters::persistence::SqlitePlayerRepository,
}
#[allow(dead_code)]
impl TestSnapshot {
    /// Parses `query` and searches the snapshot, giving the page as JSON.
    pub fn search(&self, query: &str) -> fm_savelens_backend::Result<serde_json::Value> {
        use fm_savelens_backend::{
            adapters::http::routes::players::filter::parse_filter,
            application::use_cases::players::PlayerRepository,
        };
        let page = self.players.search(&self.id, &parse_filter(query)?)?;
        Ok(serde_json::to_value(page)?)
    }
    pub fn player(
        &self,
        id: u32,
    ) -> fm_savelens_backend::Result<Option<fm_savelens_backend::domain::entities::PlayerDetail>>
    {
        use fm_savelens_backend::application::use_cases::players::PlayerRepository;
        self.players.player(&self.id, id)
    }
}

/// Builds a snapshot from a parsed save and publishes it in `dir` under a fresh id.
#[allow(dead_code)]
pub fn create_snapshot(
    dir: &Path,
    save: fm_savelens_backend::domain::entities::ParsedSave,
    cancel: &fm_savelens_backend::utils::cancel::CancelFlag,
) -> fm_savelens_backend::Result<TestSnapshot> {
    use fm_savelens_backend::{
        adapters::persistence::{SqlitePlayerRepository, SqliteSnapshotRepository},
        application::use_cases::saves::SnapshotRepository,
        domain::entities::{NewSnapshot, SnapshotId},
    };
    let job = uuid::Uuid::new_v4().simple().to_string();
    let id = SnapshotId::parse(&format!("{job}{job}"))?;
    let snapshots = SqliteSnapshotRepository::new(dir.to_path_buf());
    let snapshot = NewSnapshot::from_parsed(save, Default::default());
    snapshots.create(&job, &snapshot, cancel)?;
    snapshots.publish(&job, &id)?;
    Ok(TestSnapshot {
        path: dir.join(format!("{id}.sqlite")),
        id,
        players: SqlitePlayerRepository::new(dir.to_path_buf()),
    })
}

/// A save use case over real adapters and snapshots kept in `snapshots`.
#[allow(dead_code)]
pub fn save_use_case(
    snapshots: std::path::PathBuf,
) -> fm_savelens_backend::application::use_cases::SaveUseCase {
    use fm_savelens_backend::{
        adapters::{
            parser::reader::FmSaveReader,
            persistence::{FsSaveFiles, SqliteSnapshotRepository},
        },
        application::{
            session::{Session, SettingsRepository},
            use_cases::SaveUseCase,
        },
        domain::entities::Settings,
    };
    use std::sync::Arc;
    struct MemorySettings;
    impl SettingsRepository for MemorySettings {
        fn load(&self) -> fm_savelens_backend::Result<Settings> {
            Ok(Settings {
                folder: String::new(),
                last_snapshot: None,
                session: None,
            })
        }
        fn save(&self, _: &Settings) -> fm_savelens_backend::Result<()> {
            Ok(())
        }
    }
    SaveUseCase::new(
        Session::new(Arc::new(MemorySettings)).unwrap(),
        Arc::new(FmSaveReader),
        Arc::new(FsSaveFiles),
        Arc::new(SqliteSnapshotRepository::new(snapshots)),
    )
}

use fm_savelens_backend::{
    adapters::http::app_state::Shared,
    application::use_cases::wait_for,
    infra::{app::create_app, data_dir::DataDir, setup::init_app_state},
    Error, Result,
};
use std::net::SocketAddr;
use tokio::{net::TcpListener, task::JoinHandle};
use tokio_util::sync::CancellationToken;

#[allow(dead_code)]
pub struct RunningServer {
    pub address: SocketAddr,
    pub app: Shared,
    shutdown: CancellationToken,
    task: JoinHandle<std::io::Result<()>>,
    _data: DataDir,
}

#[allow(dead_code)]
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
#[allow(dead_code)]
pub async fn start(port: u16, data: &std::path::Path) -> Result<RunningServer> {
    let (app, data) = init_app_state(data)?;
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
