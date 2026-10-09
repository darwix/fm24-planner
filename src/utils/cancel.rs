use crate::{Error, Result};
use tokio_util::sync::CancellationToken;

/// Fails with `CANCELLED` once the import was cancelled.
pub fn check(cancel: &CancellationToken) -> Result<()> {
    if cancel.is_cancelled() {
        Err(Error::new("CANCELLED", "Import cancelled"))
    } else {
        Ok(())
    }
}
