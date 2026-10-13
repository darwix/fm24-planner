use crate::{domain::entities::SnapshotId, Error, Result};
use serde_json::Value;
use std::sync::Arc;

/// Planner state (squad arrangement, scouting kept and passed) stored inside a snapshot.
pub trait PlannerRepository: Send + Sync {
    fn get(&self, snapshot: &SnapshotId, key: &str) -> Result<Option<Value>>;
    fn put(&self, snapshot: &SnapshotId, key: &str, value: &Value) -> Result<()>;
}

pub struct PlannerUseCase {
    planner: Arc<dyn PlannerRepository>,
}
impl PlannerUseCase {
    pub fn new(planner: Arc<dyn PlannerRepository>) -> Self {
        Self { planner }
    }
    pub fn get(&self, snapshot: &str, key: &str) -> Result<Option<Value>> {
        self.planner.get(&SnapshotId::parse(snapshot)?, valid(key)?)
    }
    pub fn put(&self, snapshot: &str, key: &str, value: &Value) -> Result<()> {
        self.planner
            .put(&SnapshotId::parse(snapshot)?, valid(key)?, value)
    }
}

fn valid(key: &str) -> Result<&str> {
    if key.is_empty() || key.len() > 200 {
        return Err(Error::query("Invalid planner key."));
    }
    Ok(key)
}
