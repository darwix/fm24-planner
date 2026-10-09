use crate::{
    application::use_cases::ports::PlayerRepository,
    domain::entities::{PlayerDetail, PlayerFilter, PlayerPage, SnapshotId},
    Error, Result,
};
use std::sync::Arc;

pub struct PlayerUseCase {
    players: Arc<dyn PlayerRepository>,
}
impl PlayerUseCase {
    pub fn new(players: Arc<dyn PlayerRepository>) -> Self {
        Self { players }
    }
    pub fn search_players(&self, snapshot: &str, filter: &PlayerFilter) -> Result<PlayerPage> {
        self.players.search(&SnapshotId::parse(snapshot)?, filter)
    }
    pub fn player(&self, snapshot: &str, player: &str) -> Result<PlayerDetail> {
        let not_found = || Error::new("NOT_FOUND", "Player not found.");
        let id = player.parse().map_err(|_| not_found())?;
        self.players
            .player(&SnapshotId::parse(snapshot)?, id)?
            .ok_or_else(not_found)
    }
}
