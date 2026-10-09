mod support;
use fm_savelens_backend::{parser::parse_save, storage};
use serde_json::json;
use tokio_util::sync::CancellationToken;

#[test]
fn search_filters_contract_end_and_value_in_sql() {
    let dir = tempfile::tempdir().unwrap();
    let cancel = CancellationToken::new();
    let path = dir.path().join("s.fm");
    support::archive(&path, false, "24.3.0+0", 64);
    let save = parse_save(&path, &cancel, |_, _| {}).unwrap();
    let dbpath = dir.path().join("s.sqlite");
    storage::create_snapshot(&dbpath, &save, json!({}), &cancel).unwrap();
    rusqlite::Connection::open(&dbpath)
        .unwrap()
        .execute_batch(
            "UPDATE players SET contract_end='2053-06-30', value=4000000 WHERE id=1;
         UPDATE players SET contract_end='2053-07-01', value=9000000 WHERE id=2;
         UPDATE players SET contract_end='2056-06-30', value=1000000 WHERE id=3;",
        )
        .unwrap();
    let db = storage::open_snapshot(&dbpath).unwrap();
    let total = |q: &str| storage::search(&db, q).unwrap()["total"].as_i64().unwrap();
    // inclusive cutoff; players without a contract never match
    assert_eq!(total("contractEndMax=2053-06-30"), 1);
    assert_eq!(total("contractEndMax=2053-07-01"), 2);
    assert_eq!(total("valueMax=4000000"), 2);
    assert_eq!(total("valueMax=4000000&contractEndMax=2053-07-01"), 1);
    assert!(storage::search(&db, "contractEndMax=soon").is_err());
    assert!(storage::search(&db, "valueMax=-1").is_err());
}
