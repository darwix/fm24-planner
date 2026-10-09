use std::{collections::HashMap, sync::LazyLock};

static COUNTRIES: LazyLock<HashMap<String, String>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../assets/nations.json")).expect("country catalog")
});
pub fn nation_name(id: u32) -> String {
    COUNTRIES
        .get(&id.to_string())
        .cloned()
        .unwrap_or_else(|| format!("Nation {id}"))
}
