impl From<rusqlite::Error> for crate::Error {
    fn from(e: rusqlite::Error) -> Self {
        Self::new("DATABASE", e.to_string())
    }
}
impl From<serde_json::Error> for crate::Error {
    fn from(e: serde_json::Error) -> Self {
        Self::new("JSON", e.to_string())
    }
}
