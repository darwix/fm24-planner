mod support;
use fm_squad_planner::domain::entities::{ParsedSave, Player, CATALOG};
use fm_squad_planner::utils::cancel::CancelFlag;
use serde_json::{json, Value};

fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/position-filters.json")).unwrap()
}

fn snapshot() -> (tempfile::TempDir, support::TestSnapshot) {
    let dir = tempfile::tempdir().unwrap();
    let players = fixture()["players"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            let mut ratings = vec![0u8; CATALOG.positions.len()];
            for (index, rating) in row["ratings"].as_object().unwrap() {
                ratings[index.parse::<usize>().unwrap()] = rating.as_u64().unwrap() as u8;
            }
            serde_json::from_value::<Player>(json!({
                "id":row["id"], "uid":row["id"], "name":row["name"], "fullName":row["name"],
                "age":24, "birthDate":"2000-01-01", "nationId":1, "otherNationIds":[],
                "clubId":row["clubId"], "club":row["clubId"].as_u64().map(|id| format!("Club {id}")),
                "ca":100 + row["id"].as_u64().unwrap(), "pa":150,
                "positions":CATALOG.positions.iter().enumerate().filter(|(i, _)| ratings[*i] >= 15).map(|(_, p)| p).collect::<Vec<_>>(),
                "positionRatings":ratings,
                "attributes":CATALOG.attributes.iter().map(|a| (a.key.clone(), row["attribute"].clone())).collect::<serde_json::Map<_, _>>(),
                "rawAttributes":[], "source":{}
            })).unwrap()
        })
        .collect();
    let save = ParsedSave {
        name: "Position fixture".into(),
        format_version: "24.3.0+0".into(),
        game_date: "2024-01-01".into(),
        players,
        clubs: vec![],
        managed_club_id: None,
        diagnostics: json!({}),
        warnings: vec![],
    };
    let db = support::create_snapshot(dir.path(), save, &CancelFlag::new()).unwrap();
    (dir, db)
}

fn ids(result: &Value) -> Value {
    result["players"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["id"].clone())
        .collect()
}

#[test]
fn position_queries_match_shared_boundaries_validation_and_pagination() {
    let (_dir, db) = snapshot();
    let fixture = fixture();
    for sample in fixture["cases"].as_array().unwrap() {
        let query = sample["query"].as_str().unwrap();
        let result = db
            .search(&format!("{query}&sort=ca&direction=asc"))
            .unwrap();
        assert_eq!(result["total"], sample["total"], "{query}");
        assert_eq!(ids(&result), sample["ids"], "{query}");
    }
    for query in fixture["invalid"].as_array().unwrap() {
        assert!(db.search(query.as_str().unwrap()).is_err(), "{query}");
    }
    let descending = db
        .search("position=2,4&positionMatch=or&sort=ca&direction=desc&limit=2&page=2")
        .unwrap();
    assert_eq!(descending["total"], 5);
    assert_eq!(ids(&descending), json!([3, 2]));
    assert_eq!(db.search("").unwrap()["total"], 8);
}

#[test]
fn positions_combine_with_club_and_attributes_before_counting() {
    let (_dir, db) = snapshot();
    let query = "position=2,4&club=1&attr_pace=16";
    let and = db.search(query).unwrap();
    assert_eq!(and["total"], 1);
    assert_eq!(ids(&and), json!([1]));
    let or = db.search(&format!("{query}&positionMatch=or")).unwrap();
    assert_eq!(or["total"], 2);
}
