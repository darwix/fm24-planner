mod support;
use serde_json::{json, Value};
use std::{fs, time::Duration};

async fn ok(response: reqwest::Response) -> Value {
    let status = response.status();
    let value = response.json().await.unwrap();
    assert!(status.is_success(), "{status}: {value}");
    value
}

/// Imports a small save and returns the server, its url and the snapshot id.
async fn imported(dir: &std::path::Path) -> (support::RunningServer, String, String) {
    let games = dir.join("games");
    fs::create_dir(&games).unwrap();
    support::archive(&games.join("career.fm"), true, "24.3.0+0", 8);
    let server = support::start(0, &dir.join("data")).await.unwrap();
    let url = server.url();
    let client = reqwest::Client::new();
    ok(client
        .put(format!("{url}/api/settings"))
        .json(&json!({"folder":games}))
        .send()
        .await
        .unwrap())
    .await;
    let saves = ok(client.get(format!("{url}/api/saves")).send().await.unwrap()).await;
    let save = &saves["saves"][0];
    let mut job = ok(client
        .post(format!("{url}/api/imports"))
        .json(&json!({"saveId":save["id"]}))
        .send()
        .await
        .unwrap())
    .await;
    tokio::time::timeout(Duration::from_secs(10), async {
        while job["status"] == "running" {
            tokio::time::sleep(Duration::from_millis(10)).await;
            job = ok(client
                .get(format!("{url}/api/imports/{}", job["id"].as_str().unwrap()))
                .send()
                .await
                .unwrap())
            .await;
        }
    })
    .await
    .unwrap();
    assert_eq!(job["status"], "complete", "{job}");
    let id = job["snapshotId"].as_str().unwrap().to_owned();
    (server, url, id)
}

#[tokio::test]
async fn planner_state_is_kept_in_the_snapshot_across_restarts() {
    let dir = tempfile::tempdir().unwrap();
    let (server, url, id) = imported(dir.path()).await;
    let client = reqwest::Client::new();
    let planner = |key: &str| format!("{url}/api/snapshots/{id}/planner/{key}");

    // nothing stored yet
    let empty = ok(client.get(planner("squad:Arsenal")).send().await.unwrap()).await;
    assert_eq!(empty["value"], Value::Null);

    let squad = json!({"slots":[{"id":3,"p":"GK","xi":true,"x":50,"y":90}],"vacant":[]});
    ok(client
        .put(planner("squad:Arsenal"))
        .json(&json!({"value":squad}))
        .send()
        .await
        .unwrap())
    .await;
    ok(client
        .put(planner("passed"))
        .json(&json!({"value":[1,2]}))
        .send()
        .await
        .unwrap())
    .await;
    // overwrite replaces
    ok(client
        .put(planner("passed"))
        .json(&json!({"value":[2]}))
        .send()
        .await
        .unwrap())
    .await;
    ok(client
        .put(format!("{url}/api/session"))
        .json(&json!({"value":{"snapshotId":id,"club":"Arsenal"}}))
        .send()
        .await
        .unwrap())
    .await;

    // the bad cases
    let unknown = format!("{url}/api/snapshots/{}/planner/passed", "0".repeat(64));
    assert_eq!(
        client.get(&unknown).send().await.unwrap().status(),
        400,
        "unimported snapshot"
    );
    let long = "k".repeat(201);
    assert_eq!(
        client
            .put(planner(&long))
            .json(&json!({"value":1}))
            .send()
            .await
            .unwrap()
            .status(),
        400,
        "key too long"
    );

    server.shutdown().await.unwrap();
    let server = support::start(0, &dir.path().join("data")).await.unwrap();
    let url = server.url();
    let planner = |key: &str| format!("{url}/api/snapshots/{id}/planner/{key}");
    let got = ok(client.get(planner("squad:Arsenal")).send().await.unwrap()).await;
    assert_eq!(got["value"], squad);
    let got = ok(client.get(planner("passed")).send().await.unwrap()).await;
    assert_eq!(got["value"], json!([2]));
    let session = ok(client
        .get(format!("{url}/api/session"))
        .send()
        .await
        .unwrap())
    .await;
    assert_eq!(session["value"]["club"], "Arsenal");
    // players still searchable after the snapshot file was written to
    let page = ok(client
        .get(format!("{url}/api/snapshots/{id}/players"))
        .send()
        .await
        .unwrap())
    .await;
    assert!(page["total"].as_u64().unwrap() > 0);
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn shortlist_is_its_own_table_and_search_can_use_it() {
    let dir = tempfile::tempdir().unwrap();
    let (server, url, id) = imported(dir.path()).await;
    let client = reqwest::Client::new();
    let list = format!("{url}/api/snapshots/{id}/shortlist");
    let one = |p: u32| format!("{url}/api/snapshots/{id}/shortlist/{p}");
    let search = |q: &str| format!("{url}/api/snapshots/{id}/players?{q}");

    // the old planner `kept` copies become rows once, then the key is gone
    ok(client
        .put(format!("{url}/api/snapshots/{id}/planner/kept"))
        .json(&json!({"value":[{"id":1,"n":"A"},{"id":2,"n":"B"}]}))
        .send()
        .await
        .unwrap())
    .await;
    let got = ok(client.get(&list).send().await.unwrap()).await;
    assert_eq!(got["ids"], json!([1, 2]));
    let kept = ok(client
        .get(format!("{url}/api/snapshots/{id}/planner/kept"))
        .send()
        .await
        .unwrap())
    .await;
    assert_eq!(kept["value"], Value::Null);

    ok(client.put(one(3)).send().await.unwrap()).await;
    ok(client.put(one(3)).send().await.unwrap()).await; // adding twice is fine
    ok(client.delete(one(1)).send().await.unwrap()).await;
    let got = ok(client.get(&list).send().await.unwrap()).await;
    assert_eq!(got["ids"], json!([2, 3]));

    let page = ok(client.get(search("shortlist=1")).send().await.unwrap()).await;
    assert_eq!(page["total"], 2);
    let club = page["players"][0]["clubId"].clone();
    if let Some(club) = club.as_i64() {
        let page = ok(client
            .get(search(&format!("shortlist=1&excludeClub={club}")))
            .send()
            .await
            .unwrap())
        .await;
        assert!(page["total"].as_i64().unwrap() < 2);
    }
    server.shutdown().await.unwrap();
}
