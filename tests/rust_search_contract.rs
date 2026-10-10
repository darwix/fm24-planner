mod support;
use fm_savelens_backend::adapters::parser::parse_save;
use fm_savelens_backend::utils::cancel::CancelFlag;
use serde_json::json;

#[test]
fn search_filters_contract_end_and_value_in_sql() {
    let dir = tempfile::tempdir().unwrap();
    let cancel = CancelFlag::new();
    let path = dir.path().join("s.fm");
    support::archive(&path, false, "24.3.0+0", 64);
    let save = parse_save(&path, &cancel, |_, _| {}).unwrap();
    let db = support::create_snapshot(dir.path(), save, &cancel).unwrap();
    rusqlite::Connection::open(&db.path)
        .unwrap()
        .execute_batch(
            "UPDATE players SET value=NULL, contract_end=NULL;
         UPDATE players SET contract_end='2053-06-30', value=4000000 WHERE id=1;
         UPDATE players SET contract_end='2053-07-01', value=9000000 WHERE id=2;
         UPDATE players SET contract_end='2056-06-30', value=1000000 WHERE id=3;",
        )
        .unwrap();
    let total = |q: &str| db.search(q).unwrap()["total"].as_i64().unwrap();
    // inclusive cutoff; players without a contract never match
    assert_eq!(total("contractEndMax=2053-06-30"), 1);
    assert_eq!(total("contractEndMax=2053-07-01"), 2);
    assert_eq!(total("valueMax=4000000"), 2);
    assert_eq!(total("valueMax=4000000&contractEndMax=2053-07-01"), 1);
    assert!(db.search("contractEndMax=soon").is_err());
    assert!(db.search("valueMax=-1").is_err());
}

fn snapshot(dir: &tempfile::TempDir) -> support::TestSnapshot {
    let cancel = CancelFlag::new();
    let path = dir.path().join("t.fm");
    support::archive(&path, false, "24.3.0+0", 64);
    let save = parse_save(&path, &cancel, |_, _| {}).unwrap();
    support::create_snapshot(dir.path(), save, &cancel).unwrap()
}

#[test]
fn main_position_is_the_best_rated_position() {
    let dir = tempfile::tempdir().unwrap();
    let db = snapshot(&dir);
    let total = |q: &str| db.search(q).unwrap()["total"].as_i64().unwrap();
    // every fixture player is a striker (position 12)
    assert_eq!(total("mainPosition=12"), 64);
    assert_eq!(total("mainPosition=3"), 0);
    assert_eq!(total("mainPosition=3,12"), 64);
    assert!(db.search("mainPosition=99").is_err());
}

#[test]
fn unknown_value_and_excluded_ids() {
    let dir = tempfile::tempdir().unwrap();
    let db = snapshot(&dir);
    rusqlite::Connection::open(&db.path)
        .unwrap()
        .execute_batch(
            "UPDATE players SET value=1000000; UPDATE players SET value=NULL WHERE id IN (1,2);",
        )
        .unwrap();
    let total = |q: &str| db.search(q).unwrap()["total"].as_i64().unwrap();
    assert_eq!(total("valueMax=1000000"), 62);
    assert_eq!(total("valueMax=1000000&valueUnknown=1"), 64);
    assert_eq!(total("excludeIds=1,2,3"), 61);
    assert_eq!(total("excludeIds="), 64);
    assert!(db.search("excludeIds=1,x").is_err());
}

#[test]
fn post_body_becomes_the_same_query_as_get() {
    let q = fm_savelens_backend::adapters::http::routes::players::filter::query_from_body(&json!({
        "ageMin": 18, "mainPosition": [3, 12], "valueUnknown": true, "q": "a b", "skip": null,
    }));
    let pairs: std::collections::HashMap<_, _> = url::form_urlencoded::parse(q.as_bytes())
        .into_owned()
        .collect();
    assert_eq!(pairs["ageMin"], "18");
    assert_eq!(pairs["mainPosition"], "3,12");
    assert_eq!(pairs["valueUnknown"], "1");
    assert_eq!(pairs["q"], "a b");
    assert!(!pairs.contains_key("skip"));
}

#[test]
fn zero_value_in_the_save_is_stored_as_unknown() {
    let dir = tempfile::tempdir().unwrap();
    let db = snapshot(&dir);
    // fixture players carry a value of 0, which must not count as cheap
    let total = |q: &str| db.search(q).unwrap()["total"].as_i64().unwrap();
    assert_eq!(total("valueMax=5000000"), 0);
    assert_eq!(total("valueMax=5000000&valueUnknown=1"), 64);
}

#[test]
fn no_contract_keeps_only_players_without_a_contract_end_or_wage() {
    let dir = tempfile::tempdir().unwrap();
    let db = snapshot(&dir);
    rusqlite::Connection::open(&db.path)
        .unwrap()
        .execute_batch(
            "UPDATE players SET contract_end=NULL, wage=NULL;
         UPDATE players SET contract_end='2056-06-30' WHERE id=3;
         UPDATE players SET wage=500 WHERE id=4;",
        )
        .unwrap();
    let total = |q: &str| db.search(q).unwrap()["total"].as_i64().unwrap();
    // a wage with no decoded end date is still a contract
    assert_eq!(total("noContract=1"), 62);
    assert_eq!(total(""), 64);
}
