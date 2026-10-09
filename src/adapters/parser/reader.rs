use super::{parse_save, PARSER_VERSION};
use crate::utils::cancel::CancelFlag;
use crate::{application::use_cases::ports::SaveReader, domain::entities::ParsedSave, Result};
use std::path::Path;

/// Reads FM24 `.fm` saves.
pub struct FmSaveReader;

impl SaveReader for FmSaveReader {
    fn version(&self) -> &str {
        PARSER_VERSION
    }
    fn read(
        &self,
        path: &Path,
        cancel: &CancelFlag,
        progress: &dyn Fn(u8, &str),
    ) -> Result<ParsedSave> {
        parse_save(path, cancel, progress)
    }
}
