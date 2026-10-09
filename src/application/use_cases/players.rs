use crate::{
    application::use_cases::ports::PlayerRepository, domain::entities::SnapshotId, Error, Result,
};
use serde_json::Value;
use std::sync::Arc;

pub struct PlayerUseCase {
    players: Arc<dyn PlayerRepository>,
}
impl PlayerUseCase {
    pub fn new(players: Arc<dyn PlayerRepository>) -> Self {
        Self { players }
    }
    pub fn search_players(&self, snapshot: &str, query: &str) -> Result<Value> {
        self.players.search(&SnapshotId::parse(snapshot)?, query)
    }
    pub fn player(&self, snapshot: &str, player: &str) -> Result<Value> {
        let player = player
            .parse()
            .map_err(|_| Error::new("NOT_FOUND", "Player not found."))?;
        self.players.player(&SnapshotId::parse(snapshot)?, player)
    }
}
