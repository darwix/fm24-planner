use crate::{application::Workspace, Error, Result};
use serde_json::Value;
use std::{path::PathBuf, sync::Arc};

pub struct PlayerUseCase {
    ws: Arc<Workspace>,
}
impl PlayerUseCase {
    pub fn new(ws: Arc<Workspace>) -> Self {
        Self { ws }
    }
    fn snapshot_path(&self, id: &str) -> Result<PathBuf> {
        if id.len() != 64
            || !id
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        {
            return Err(Error::query("Invalid snapshot."));
        }
        let path = self.ws.data.join("snapshots").join(format!("{id}.sqlite"));
        if !path.is_file() {
            return Err(Error::query("This save has not been imported."));
        }
        Ok(path)
    }
    pub fn snapshot(&self, id: &str) -> Result<Value> {
        self.ws.store.metadata(&self.snapshot_path(id)?)
    }
    pub fn search_players(&self, id: &str, query: &str) -> Result<Value> {
        self.ws.store.search(&self.snapshot_path(id)?, query)
    }
    pub fn player(&self, id: &str, player: &str) -> Result<Value> {
        let player = player
            .parse()
            .map_err(|_| Error::new("NOT_FOUND", "Player not found."))?;
        self.ws.store.player(&self.snapshot_path(id)?, player)
    }
}
