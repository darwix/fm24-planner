mod support;
use fm_savelens_backend::{
    adapters::{parser::parse_save, persistence::SqliteSnapshotRepository},
    application::use_cases::saves::SnapshotRepository,
    domain::entities::{NewSnapshot, SnapshotId, SnapshotMeta},
    utils::cancel::CancelFlag,
};
use serde_json::{json, Value};
use std::{fs, path::Path, time::Duration};

/// Publishes a snapshot of a small save under `id`, recorded as read from `source`.
fn publish(repo: &SqliteSnapshotRepository, save: &Path, id: char, source: &str) -> SnapshotId {
    let cancel = CancelFlag::new();
    let parsed = parse_save(save, &cancel, |_, _| {}).unwrap();
    let snapshot = NewSnapshot::from_parsed(
        parsed,
        SnapshotMeta {
            source_path: source.to_owned(),
            ..Default::default()
        },
    );
    let job = format!("job-{id}");
    let id = SnapshotId::parse(&id.to_string().repeat(64)).unwrap();
    repo.create(&job, &snapshot, &cancel).unwrap();
    repo.publish(&job, &id).unwrap();
    id
}

#[test]
fn prune_removes_only_older_snapshots_of_the_same_save() {
    let dir = tempfile::tempdir().unwrap();
    let save = dir.path().join("career.fm");
    support::archive(&save, false, "24.3.0+0", 8);
    let snapshots = dir.path().join("snapshots");
    fs::create_dir(&snapshots).unwrap();
    let repo = SqliteSnapshotRepository::new(snapshots.clone());
    let old = publish(&repo, &save, 'a', "C:/games/career.fm");
    let keep = publish(&repo, &save, 'b', "C:/games/career.fm");
    let other = publish(&repo, &save, 'c', "C:/games/other.fm");
    // Files the repository cannot read as snapshots are never touched.
    fs::write(
        snapshots.join(format!("{}.sqlite", "d".repeat(64))),
        b"not sqlite",
    )
    .unwrap();
    fs::write(snapshots.join("job.partial.sqlite"), b"in progress").unwrap();

    assert_eq!(repo.prune(&keep, "C:/games/career.fm").unwrap(), 1);
    assert!(!repo.exists(&old));
    assert!(repo.exists(&keep));
    assert!(repo.exists(&other));
    assert_eq!(fs::read_dir(&snapshots).unwrap().count(), 4);
    // Nothing left to prune.
    assert_eq!(repo.prune(&keep, "C:/games/career.fm").unwrap(), 0);
}

async fn value(response: reqwest::Response) -> Value {
    let status = response.status();
    let value = response.json().await.unwrap();
    assert!(status.is_success(), "{status}: {value}");
    value
}

/// Imports the save named `name` and waits for it; returns its snapshot id.
async fn import(client: &reqwest::Client, url: &str, name: &str) -> String {
    let saves = value(client.get(format!("{url}/api/saves")).send().await.unwrap()).await;
    let save = saves["saves"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["name"] == name)
        .unwrap()
        .clone();
    let mut job = value(
        client
            .post(format!("{url}/api/imports"))
            .json(&json!({"saveId":save["id"]}))
            .send()
            .await
            .unwrap(),
    )
    .await;
    tokio::time::timeout(Duration::from_secs(10), async {
        while job["status"] == "running" {
            tokio::time::sleep(Duration::from_millis(10)).await;
            job = value(
                client
                    .get(format!("{url}/api/imports/{}", job["id"].as_str().unwrap()))
                    .send()
                    .await
                    .unwrap(),
            )
            .await;
        }
    })
    .await
    .expect("Import did not complete");
    assert_eq!(job["status"], "complete", "{job}");
    save["snapshotId"].as_str().unwrap().to_owned()
}

fn published(data: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(data.join("snapshots"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[tokio::test]
async fn reimporting_a_changed_save_replaces_its_old_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let games = dir.path().join("games");
    fs::create_dir(&games).unwrap();
    support::archive(&games.join("career.fm"), false, "24.3.0+0", 8);
    support::archive(&games.join("other.fm"), false, "24.3.0+0", 8);
    let data = dir.path().join("data");
    let server = support::start(0, &data).await.unwrap();
    let url = server.url();
    let client = reqwest::Client::new();
    value(
        client
            .put(format!("{url}/api/settings"))
            .json(&json!({"folder":games}))
            .send()
            .await
            .unwrap(),
    )
    .await;

    let first = import(&client, &url, "career.fm").await;
    let other = import(&client, &url, "other.fm").await;
    assert_eq!(published(&data).len(), 2);

    // FM saving again changes size and modification time: a new snapshot id.
    support::archive(&games.join("career.fm"), false, "24.3.0+0", 9);
    let second = import(&client, &url, "career.fm").await;
    assert_ne!(first, second);
    let mut expected = vec![format!("{second}.sqlite"), format!("{other}.sqlite")];
    expected.sort();
    assert_eq!(published(&data), expected);
    server.shutdown().await.unwrap();
}
