use super::*;

impl AppState {
    pub(super) fn list(&self, folder: &str) -> Result<Vec<SaveFile>> {
        if folder.is_empty() {
            return Ok(vec![]);
        }
        if !Path::new(folder).is_dir() {
            return Err(Error::query(
                "The save folder is unavailable. Choose an existing folder in Settings.",
            ));
        }
        let mut saves = vec![];
        for entry in fs::read_dir(folder)? {
            let entry = entry?;
            if !entry.file_type()?.is_file()
                || !entry
                    .path()
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("fm"))
            {
                continue;
            }
            let stat = entry.metadata()?;
            let modified = mtime(&stat)?;
            let path = entry.path().to_string_lossy().into_owned();
            let snapshot_id = digest(&format!(
                "{path}|{}|{modified}|{PARSER_VERSION}",
                stat.len()
            ));
            saves.push(SaveFile {
                id: digest(&path),
                name: entry.file_name().to_string_lossy().into_owned(),
                size: stat.len(),
                mtime: modified,
                modified: DateTime::<Utc>::from(stat.modified()?)
                    .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                cached: self
                    .data
                    .join("snapshots")
                    .join(format!("{snapshot_id}.sqlite"))
                    .is_file(),
                snapshot_id,
            });
        }
        saves.sort_by(|a, b| b.modified.cmp(&a.modified));
        Ok(saves)
    }
    pub(super) fn db_path(&self, id: &str) -> Result<PathBuf> {
        if id.len() != 64
            || !id
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        {
            return Err(Error::query("Invalid snapshot."));
        }
        let path = self.data.join("snapshots").join(format!("{id}.sqlite"));
        if !path.is_file() {
            return Err(Error::query("This save has not been imported."));
        }
        Ok(path)
    }
}
