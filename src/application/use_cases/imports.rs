use crate::utils::cancel::CancelFlag;
use crate::{
    application::use_cases::{
        session::{Active, Completion, Inner},
        snapshot_builder::build_snapshot,
        SaveUseCase,
    },
    domain::entities::{Job, JobStatus, SaveFile, SnapshotId, SnapshotMeta},
    utils::cancel::check,
    Error, Result,
};
use chrono::Utc;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Condvar, Mutex},
    time::Instant,
};

/// Finished jobs kept for the API; the oldest is dropped beyond this.
const MAX_JOBS: usize = 100;

impl SaveUseCase {
    /// Starts reading save `save_id` on a worker thread, or completes at once when the save is
    /// already cached.
    pub fn start_import(self: &Arc<Self>, save_id: &str) -> Result<Job> {
        let mut inner = self.session.inner.lock().unwrap();
        if inner.shutting_down {
            return Err(Error::new("SHUTTING_DOWN", "The app is shutting down."));
        }
        if inner.active.is_some() {
            return Err(Error::query(
                "Another save is already being read. Cancel it or wait for it to finish.",
            ));
        }
        let save = self
            .list(&inner.settings.folder)?
            .into_iter()
            .find(|s| s.id == save_id)
            .ok_or_else(|| {
                Error::query(
                    "The selected save is no longer in this folder. Refresh the save list.",
                )
            })?;
        let job = new_job(uuid::Uuid::new_v4().to_string(), &save);
        if inner.jobs.len() >= MAX_JOBS {
            drop_oldest_job(&mut inner);
        }
        if save.cached {
            self.remember_snapshot(&mut inner, &save.snapshot_id)?;
        } else {
            self.spawn_import(&mut inner, &job.id, save)?;
        }
        inner.jobs.insert(job.id.clone(), job.clone());
        Ok(job)
    }

    fn spawn_import(self: &Arc<Self>, inner: &mut Inner, id: &str, save: SaveFile) -> Result<()> {
        let active = Active {
            id: id.to_owned(),
            cancel: CancelFlag::new(),
            done: Arc::new((Mutex::new(false), Condvar::new())),
        };
        inner.active = Some(active.clone());
        let file = Path::new(&inner.settings.folder).join(&save.name);
        let app = self.clone();
        if let Err(error) = std::thread::Builder::new()
            .name("save-import".into())
            .spawn(move || app.run_import(active, file, save))
        {
            inner.active = None;
            return Err(error.into());
        }
        Ok(())
    }

    /// Worker thread: runs the import, records how it ended, and signals completion.
    fn run_import(self: Arc<Self>, active: Active, file: PathBuf, save: SaveFile) {
        let start = Instant::now();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.import(&active, &file, &save, start)
        }))
        .unwrap_or_else(|_| {
            Err(Error::new(
                "IMPORT_FAILED",
                "The reader stopped unexpectedly.",
            ))
        });
        let cleanup = self.snapshots.discard(&active.id);
        {
            let mut inner = self.session.inner.lock().unwrap();
            if let Err(e) = result.and(cleanup) {
                if let Some(job) = inner.jobs.get_mut(&active.id) {
                    job.status = if active.cancel.is_cancelled() {
                        JobStatus::Cancelled
                    } else {
                        JobStatus::Error
                    };
                    job.message = e.message;
                }
            }
            inner.active = None;
        }
        let (lock, done) = &*active.done;
        *lock.lock().unwrap() = true;
        done.notify_all();
    }

    /// Reads the save, writes the snapshot and publishes it. Returns the player count.
    fn import(
        &self,
        active: &Active,
        file: &Path,
        save: &SaveFile,
        start: Instant,
    ) -> Result<usize> {
        check(&active.cancel)?;
        let before = self.files.stat(file)?;
        if before.size != save.size || before.mtime != save.mtime {
            return Err(Error::new(
                "SAVE_CHANGED",
                "The save changed before import began. Refresh the save list and retry.",
            ));
        }
        let parsed = self
            .reader
            .read(file, &active.cancel, &|progress, message| {
                let mut inner = self.session.inner.lock().unwrap();
                if let Some(job) = inner.jobs.get_mut(&active.id) {
                    job.progress = progress;
                    job.message = message.into();
                }
            })?;
        let players = parsed.players.len();
        let snapshot = build_snapshot(
            parsed,
            SnapshotMeta {
                snapshot_id: save.snapshot_id.clone(),
                source_path: file.to_string_lossy().into_owned(),
                source_name: save.name.clone(),
                source_size: save.size,
                source_mtime: save.mtime,
                parser_version: self.reader.version().to_owned(),
                ..Default::default()
            },
        );
        self.snapshots
            .create(&active.id, &snapshot, &active.cancel)?;
        let after = self.files.stat(file)?;
        if after.size != before.size || after.modified != before.modified {
            return Err(Error::new(
                "SAVE_CHANGED",
                "The save changed during import. Wait for saving to finish and retry.",
            ));
        }
        // Serialize final publication with cancellation and settings changes.
        let mut inner = self.session.inner.lock().unwrap();
        check(&active.cancel)?;
        let snapshot_id = SnapshotId::parse(&save.snapshot_id)?;
        self.snapshots.publish(&active.id, &snapshot_id)?;
        if let Err(error) = self.remember_snapshot(&mut inner, &save.snapshot_id) {
            self.snapshots.unpublish(&snapshot_id)?;
            return Err(error);
        }
        let job = inner.jobs.get_mut(&active.id).unwrap();
        job.status = JobStatus::Complete;
        job.progress = 100;
        job.message = "Ready".into();
        job.player_count = Some(players);
        job.elapsed_ms = Some(start.elapsed().as_millis());
        Ok(players)
    }

    /// Saves `snapshot_id` as the last opened snapshot; the session changes only once it is stored.
    fn remember_snapshot(&self, inner: &mut Inner, snapshot_id: &str) -> Result<()> {
        let mut settings = inner.settings.clone();
        settings.last_snapshot = Some(snapshot_id.to_owned());
        self.session.persist(&settings)?;
        inner.settings = settings;
        Ok(())
    }

    pub fn job(&self, id: &str) -> Result<Job> {
        self.session
            .inner
            .lock()
            .unwrap()
            .jobs
            .get(id)
            .cloned()
            .ok_or_else(|| Error::new("NOT_FOUND", "Import not found."))
    }
    pub fn cancel(&self, id: &str) -> Result<Job> {
        let active = {
            let inner = self.session.inner.lock().unwrap();
            if !inner.jobs.contains_key(id) {
                return Err(Error::new("NOT_FOUND", "Import not found."));
            }
            inner
                .active
                .as_ref()
                .filter(|a| a.id == id && inner.jobs[id].status == JobStatus::Running)
                .cloned()
                .inspect(|a| a.cancel.cancel())
        };
        if let Some(active) = active {
            wait_for(&active.done);
        }
        Ok(self.session.inner.lock().unwrap().jobs[id].clone())
    }
    pub fn stop_import(&self) {
        let active = self.session.inner.lock().unwrap().active.clone();
        if let Some(active) = active {
            active.cancel.cancel();
            wait_for(&active.done);
        }
    }
    pub fn begin_shutdown(&self) -> Option<Completion> {
        let mut inner = self.session.inner.lock().unwrap();
        // Use the import/publication mutex so no late request can start work
        // between capturing the active worker and stopping HTTP acceptance.
        inner.shutting_down = true;
        inner.active.as_ref().map(|active| {
            active.cancel.cancel();
            active.done.clone()
        })
    }
}

/// A new job for `save`: already complete when the save is cached.
fn new_job(id: String, save: &SaveFile) -> Job {
    let (status, progress, message) = if save.cached {
        (JobStatus::Complete, 100, "Loaded from local cache")
    } else {
        (JobStatus::Running, 0, "Starting import")
    };
    Job {
        id,
        save_id: save.id.clone(),
        snapshot_id: save.snapshot_id.clone(),
        status,
        progress,
        message: message.into(),
        started_at: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        player_count: None,
        elapsed_ms: None,
    }
}

fn drop_oldest_job(inner: &mut Inner) {
    if let Some(key) = inner
        .jobs
        .values()
        .min_by_key(|j| &j.started_at)
        .map(|j| j.id.clone())
    {
        inner.jobs.remove(&key);
    }
}

pub fn wait_for(completion: &Completion) {
    let (lock, done) = &**completion;
    let mut complete = lock.lock().unwrap();
    while !*complete {
        complete = done.wait(complete).unwrap();
    }
}
