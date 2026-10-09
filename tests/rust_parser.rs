mod support;
use fm_savelens_backend::utils::cancel::CancelFlag;
use fm_savelens_backend::{
    adapters::parser::{archive::Archive, parse_save, PARSER_VERSION},
    adapters::persistence::SqliteSnapshotRepository,
    application::use_cases::saves::SnapshotRepository,
    domain::{
        catalog::CATALOG,
        entities::{NewSnapshot, SnapshotId, SnapshotMeta},
    },
};
use serde_json::json;
use std::fs;

#[test]
fn snapshot_metadata_preserves_fractional_source_timestamps() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("timestamp.fm");
    support::archive(&path, false, "24.3.0+0", 64);
    let file = fs::OpenOptions::new().write(true).open(&path).unwrap();
    // This timestamp loses precision with serde_json's default f64 deserializer.
    let modified = std::time::UNIX_EPOCH + std::time::Duration::new(1_788_551_603, 590_001_300);
    file.set_times(fs::FileTimes::new().set_modified(modified))
        .unwrap();
    let stat = file.metadata().unwrap();
    let source_mtime = fm_savelens_backend::utils::files::mtime(&stat).unwrap();
    let cancel = CancelFlag::new();
    let save = parse_save(&path, &cancel, |_, _| {}).unwrap();
    let snapshots = dir.path().join("snapshots");
    fs::create_dir(&snapshots).unwrap();
    let id = "b".repeat(64);
    let repo = SqliteSnapshotRepository::new(snapshots.clone());
    let snapshot = NewSnapshot::from_parsed(
        save,
        SnapshotMeta {
            source_path: path.to_string_lossy().into_owned(),
            source_size: stat.len(),
            source_mtime,
            parser_version: PARSER_VERSION.to_owned(),
            ..Default::default()
        },
    );
    repo.create("job", &snapshot, &cancel).unwrap();
    repo.publish("job", &SnapshotId::parse(&id).unwrap())
        .unwrap();
    let saves = support::save_use_case(snapshots);
    let meta = saves.snapshot(&id).unwrap();
    assert_eq!(meta.source_mtime, source_mtime);
    assert!(!meta.stale);

    // A timestamp-only change must still invalidate the snapshot at the same size.
    file.set_times(
        fs::FileTimes::new().set_modified(modified + std::time::Duration::from_millis(1)),
    )
    .unwrap();
    assert_eq!(file.metadata().unwrap().len(), stat.len());
    assert_eq!(saves.snapshot(&id).unwrap().stale, true);
}

#[test]
fn compressed_and_plain_records_match_and_query_correctly() {
    let dir = tempfile::tempdir().unwrap();
    let cancel = CancelFlag::new();
    let mut variants = vec![];
    for compressed in [false, true] {
        let path = dir.path().join(format!("{compressed}.fm"));
        support::archive(&path, compressed, "24.3.0+0", 64);
        let before = fs::read(&path).unwrap();
        let save = parse_save(&path, &cancel, |_, _| {}).unwrap();
        assert_eq!(save.players.len(), 64);
        assert_eq!(save.players[0].name, "Álvaro García");
        assert_eq!(save.players[0].uid, 2_000_000_000);
        assert_eq!(save.players[0].ca, 200);
        assert_eq!(save.players[0].pa, 199);
        assert_eq!(save.players[0].birth_date, "2000-02-29");
        assert_eq!(save.players[0].age, 24);
        assert_eq!(save.players[0].other_nation_ids, vec![12]);
        assert_eq!(save.players[0].positions, vec!["ST"]);
        assert_eq!(save.players[0].attributes.len(), 62);
        for a in &CATALOG.attributes {
            assert_eq!(
                save.players[0].attributes[&a.key],
                if a.scale == 5 { 13 } else { 10 }
            );
        }
        assert_eq!(save.clubs.len(), 1);
        assert!(save.players[0].club.is_none());
        assert_eq!(save.diagnostics["ambiguous"], 0);
        assert_eq!(save.diagnostics["unbound"], 0);
        assert_eq!(fs::read(&path).unwrap(), before);
        variants.push(serde_json::to_value(&save.players).unwrap());
        let db = support::create_snapshot(dir.path(), save, &cancel).unwrap();
        let first = db
            .search("q=alvaro&nation=12&position=12&attr_pace=13&caMin=200&paMax=199&limit=25")
            .unwrap();
        assert_eq!(first["total"], 64);
        assert_eq!(first["players"].as_array().unwrap().len(), 25);
        let second = db.search("limit=25&page=2").unwrap();
        assert_ne!(first["players"][0]["id"], second["players"][0]["id"]);
        assert_eq!(db.search("q=%25").unwrap()["total"], 0);
        assert_eq!(db.search("club=-1").unwrap()["total"], 64);
        for field in ["age", "ca", "pa"] {
            assert_eq!(db.search(&format!("{field}Min=250")).unwrap()["total"], 0);
            assert_eq!(db.search(&format!("{field}Max=250")).unwrap()["total"], 64);
        }
        for q in [
            "paMin=-1",
            "caMax=9007199254740992",
            "ageMin=1.5",
            "ageMin=30&ageMax=15",
            "sort=invalid",
            "direction=sideways",
            "limit=0",
            "attr_pace=21",
        ] {
            assert!(db.search(q).is_err(), "{q}");
        }
        assert!(db.player(99999).unwrap().is_none());
    }
    assert_eq!(variants[0], variants[1]);
}

#[test]
fn cached_timestamps_round_trip_and_staleness_still_detects_source_changes() {
    use fm_savelens_backend::{adapters::parser::PARSER_VERSION, utils::files::mtime};
    use rusqlite::Connection;
    use serde_json::Value;
    use std::time::{Duration, UNIX_EPOCH};

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("unchanged.fm");
    fs::write(&path, b"unchanged source").unwrap();
    let file = fs::OpenOptions::new().write(true).open(&path).unwrap();
    let stamp = UNIX_EPOCH + Duration::new(1_780_000_000, 100_000);
    file.set_times(fs::FileTimes::new().set_modified(stamp))
        .unwrap();
    let stat = fs::metadata(&path).unwrap();
    let original = mtime(&stat).unwrap();
    // This decimal previously deserialized to 1780000000000.1 instead.
    let legacy: Value = serde_json::from_str("{\"sourceMtime\":1780000000000.0999}").unwrap();
    assert_eq!(legacy["sourceMtime"].as_f64(), Some(1780000000000.0999));
    let metadata = json!({
        "sourcePath":path, "sourceSize":stat.len(), "sourceMtime":original,
        "parserVersion":PARSER_VERSION,
    });
    let encoded = metadata.to_string();
    assert_eq!(
        serde_json::from_str::<Value>(&encoded).unwrap()["sourceMtime"].as_f64(),
        Some(original)
    );
    let snapshots = dir.path().join("snapshots");
    fs::create_dir(&snapshots).unwrap();
    let id = "c".repeat(64);
    let dbpath = snapshots.join(format!("{id}.sqlite"));
    {
        let db = Connection::open(&dbpath).unwrap();
        db.execute_batch("CREATE TABLE metadata (key TEXT PRIMARY KEY,value TEXT NOT NULL)")
            .unwrap();
        db.execute("INSERT INTO metadata VALUES ('snapshot',?)", [&encoded])
            .unwrap();
    }
    let saves = support::save_use_case(snapshots);
    assert_eq!(saves.snapshot(&id).unwrap().stale, false);
    file.set_times(fs::FileTimes::new().set_modified(stamp + Duration::from_secs(1)))
        .unwrap();
    assert_eq!(saves.snapshot(&id).unwrap().stale, true);
    file.set_times(fs::FileTimes::new().set_modified(stamp))
        .unwrap();
    assert_eq!(saves.snapshot(&id).unwrap().stale, false);
    file.set_len(stat.len() + 1).unwrap();
    file.set_times(fs::FileTimes::new().set_modified(stamp))
        .unwrap();
    assert_eq!(saves.snapshot(&id).unwrap().stale, true);
    file.set_len(stat.len()).unwrap();
    file.set_times(fs::FileTimes::new().set_modified(stamp))
        .unwrap();
    assert_eq!(saves.snapshot(&id).unwrap().stale, false);
    let mut incompatible = metadata;
    incompatible["parserVersion"] = json!("old-parser");
    let writer = Connection::open(&dbpath).unwrap();
    writer
        .execute(
            "UPDATE metadata SET value=? WHERE key='snapshot'",
            [incompatible.to_string()],
        )
        .unwrap();
    assert_eq!(saves.snapshot(&id).unwrap().stale, true);
    writer
        .execute(
            "UPDATE metadata SET value=? WHERE key='snapshot'",
            [encoded],
        )
        .unwrap();
    drop(writer);
    assert_eq!(saves.snapshot(&id).unwrap().stale, false);
    drop(file);
    fs::remove_file(&path).unwrap();
    assert_eq!(saves.snapshot(&id).unwrap().stale, true);
}
#[test]
fn malformed_and_unsupported_archives_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let cancel = CancelFlag::new();
    let path = dir.path().join("bad.fm");
    for size in [0, 6, 25, 26, 100] {
        fs::write(&path, vec![0; size]).unwrap();
        assert!(Archive::open(&path, &cancel).is_err());
    }
    support::archive(&path, true, "23.4.0+0", 1);
    assert_eq!(
        parse_save(&path, &cancel, |_, _| {}).err().unwrap().code,
        "UNSUPPORTED_VERSION"
    );
    support::write_archive(&path, false, vec![("game_db", vec![0; 40])]);
    assert_eq!(
        parse_save(&path, &cancel, |_, _| {}).err().unwrap().code,
        "MISSING_MEMBER"
    );
    support::archive(&path, false, "24.3.0+0", 1);
    let mut bytes = fs::read(&path).unwrap();
    bytes[9..17].fill(255);
    fs::write(&path, &bytes).unwrap();
    assert_eq!(
        Archive::open(&path, &cancel).err().unwrap().code,
        "INVALID_SIZE"
    );
}
#[test]
fn foreign_ability_ownership_is_excluded_and_sorting_is_directional() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ownership.fm");
    let mut database = support::database(64);
    let first_block = database.len() - 64 * 512;
    database[first_block..first_block + 4].copy_from_slice(&9999u32.to_le_bytes());
    // Give the second player a different CA so sorting has a meaningful result.
    database[first_block + 512 + 19] = 100;
    let mut info = vec![0; 32];
    info[12..20].copy_from_slice(b"24.3.0+0");
    support::write_archive(
        &path,
        false,
        vec![("game_info", info), ("game_db", database)],
    );
    let cancel = CancelFlag::new();
    let save = parse_save(&path, &cancel, |_, _| {}).unwrap();
    assert_eq!(save.players.len(), 63);
    assert!(!save.players.iter().any(|p| p.id == 1));
    assert_eq!(save.players.iter().find(|p| p.id == 2).unwrap().ca, 100);
    let db = support::create_snapshot(dir.path(), save, &cancel).unwrap();
    assert_eq!(
        db.search("sort=ca&direction=asc").unwrap()["players"][0]["id"],
        2
    );
    assert_ne!(
        db.search("sort=ca&direction=desc").unwrap()["players"][0]["id"],
        2
    );
}
#[test]
fn cancellation_and_changed_source_never_produce_success() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("save.fm");
    support::archive(&path, true, "24.3.0+0", 2000);
    let cancel = CancelFlag::new();
    let result = parse_save(&path, &cancel, |progress, _| {
        if progress >= 35 {
            cancel.cancel()
        }
    });
    assert_eq!(result.err().unwrap().code, "CANCELLED");
    let cancel = CancelFlag::new();
    let result = parse_save(&path, &cancel, |progress, _| {
        if progress == 52 {
            let file = fs::OpenOptions::new().write(true).open(&path).unwrap();
            file.set_len(file.metadata().unwrap().len() + 1).unwrap();
        }
    });
    assert_eq!(result.err().unwrap().code, "SAVE_CHANGED");
}
