use crate::{Error, Result};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

/// Shared flag an import checks to stop early. Clones share the flag.
#[derive(Clone, Default)]
pub struct CancelFlag(Arc<AtomicBool>);
impl CancelFlag {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

/// Fails with `CANCELLED` once the import was cancelled.
pub fn check(cancel: &CancelFlag) -> Result<()> {
    if cancel.is_cancelled() {
        Err(Error::new("CANCELLED", "Import cancelled"))
    } else {
        Ok(())
    }
}
