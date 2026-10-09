use crate::{
    application::use_cases::saves::SaveFiles,
    domain::entities::SourceFile,
    utils::files::{canonical, mtime},
    Error, Result,
};
use std::{fs, path::Path};

/// `.fm` files on the local disk.
pub struct FsSaveFiles;

fn source(path: &Path, stat: &fs::Metadata) -> Result<SourceFile> {
    Ok(SourceFile {
        name: path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        path: path.to_path_buf(),
        size: stat.len(),
        mtime: mtime(stat)?,
        modified: stat.modified()?,
    })
}

impl SaveFiles for FsSaveFiles {
    fn scan(&self, folder: &str) -> Result<Vec<SourceFile>> {
        if !Path::new(folder).is_dir() {
            return Err(Error::query(
                "The save folder is unavailable. Choose an existing folder in Settings.",
            ));
        }
        let mut found = vec![];
        for entry in fs::read_dir(folder)? {
            let entry = entry?;
            let is_save = entry.file_type()?.is_file()
                && entry
                    .path()
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("fm"));
            if is_save {
                found.push(source(&entry.path(), &entry.metadata()?)?);
            }
        }
        Ok(found)
    }
    fn stat(&self, path: &Path) -> Result<SourceFile> {
        source(path, &fs::metadata(path)?)
    }
    fn directory(&self, folder: &str) -> Result<String> {
        if !Path::new(folder).is_dir() {
            return Err(Error::query("Choose an existing save folder."));
        }
        Ok(canonical(Path::new(folder))?.to_string_lossy().into_owned())
    }
}
