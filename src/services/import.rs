use super::*;

impl AppState {
    pub(super) fn start_import(self: &Arc<Self>, save_id: &str) -> Result<Job> {
        let mut inner = self.inner.lock().unwrap();
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
        let id = uuid::Uuid::new_v4().to_string();
        let job = Job {
            id: id.clone(),
            save_id: save_id.into(),
            snapshot_id: save.snapshot_id.clone(),
            status: if save.cached { "complete" } else { "running" }.into(),
            progress: if save.cached { 100 } else { 0 },
            message: if save.cached {
                "Loaded from local cache"
            } else {
                "Starting import"
            }
            .into(),
            started_at: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            player_count: None,
            elapsed_ms: None,
        };
        if inner.jobs.len() >= 100 {
            if let Some(key) = inner
                .jobs
                .values()
                .min_by_key(|j| &j.started_at)
                .map(|j| j.id.clone())
            {
                inner.jobs.remove(&key);
            }
        }
        if save.cached {
            let mut settings = inner.settings.clone();
            settings.last_snapshot = Some(save.snapshot_id);
            self.persist(&settings)?;
            inner.settings = settings;
        } else {
            let active = Active {
                id: id.clone(),
                cancel: CancellationToken::new(),
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
        }
        inner.jobs.insert(id, job.clone());
        Ok(job)
    }
    pub(super) fn run_import(self: Arc<Self>, active: Active, file: PathBuf, save: SaveFile) {
        let start = Instant::now();
        let temporary = self
            .data
            .join("snapshots")
            .join(format!("{}.partial.sqlite", active.id));
        let result=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||->Result<usize>{
            check(&active.cancel)?;
            let before=fs::metadata(&file)?;
            if before.len()!=save.size || mtime(&before)?!=save.mtime{return Err(Error::new("SAVE_CHANGED","The save changed before import began. Refresh the save list and retry."))}
            let parsed=parser::parse_save(&file,&active.cancel,|progress,message|{let mut inner=self.inner.lock().unwrap();if let Some(job)=inner.jobs.get_mut(&active.id){job.progress=progress;job.message=message.into();}})?;
            storage::create_snapshot(&temporary,&parsed,json!({"snapshotId":save.snapshot_id,"sourcePath":file,"sourceName":save.name,"sourceSize":save.size,"sourceMtime":save.mtime,"parserVersion":PARSER_VERSION}),&active.cancel)?;
            let after=fs::metadata(&file)?;
            if after.len()!=before.len() || after.modified()?!=before.modified()? {return Err(Error::new("SAVE_CHANGED","The save changed during import. Wait for saving to finish and retry."))}
            // Serialize final publication with cancellation and settings changes.
            let mut inner=self.inner.lock().unwrap();check(&active.cancel)?;
            let destination=self.data.join("snapshots").join(format!("{}.sqlite",save.snapshot_id));
            fs::rename(&temporary,&destination)?;
            let mut settings=inner.settings.clone();settings.last_snapshot=Some(save.snapshot_id.clone());if let Err(error)=self.persist(&settings){fs::remove_file(&destination)?;return Err(error)}inner.settings=settings;
            let job=inner.jobs.get_mut(&active.id).unwrap();job.status="complete".into();job.progress=100;job.message="Ready".into();job.player_count=Some(parsed.players.len());job.elapsed_ms=Some(start.elapsed().as_millis());
            Ok(parsed.players.len())
        })).unwrap_or_else(|_|Err(Error::new("IMPORT_FAILED","The reader stopped unexpectedly.")));
        let cleanup = if temporary.exists() {
            fs::remove_file(&temporary)
        } else {
            Ok(())
        };
        {
            let mut inner = self.inner.lock().unwrap();
            if let Err(e) = result.and_then(|_| cleanup.map_err(Error::from)) {
                if let Some(job) = inner.jobs.get_mut(&active.id) {
                    job.status = if active.cancel.is_cancelled() {
                        "cancelled"
                    } else {
                        "error"
                    }
                    .into();
                    job.message = e.message;
                }
            }
            inner.active = None;
        }
        let (lock, done) = &*active.done;
        *lock.lock().unwrap() = true;
        done.notify_all();
    }
    pub(super) fn cancel(&self, id: &str) -> Result<Job> {
        let active = {
            let inner = self.inner.lock().unwrap();
            if !inner.jobs.contains_key(id) {
                return Err(Error::new("NOT_FOUND", "Import not found."));
            }
            inner
                .active
                .as_ref()
                .filter(|a| a.id == id && inner.jobs[id].status == "running")
                .cloned()
                .inspect(|a| a.cancel.cancel())
        };
        if let Some(active) = active {
            wait_for(&active.done);
        }
        Ok(self.inner.lock().unwrap().jobs[id].clone())
    }
    pub fn stop_import(&self) {
        let active = self.inner.lock().unwrap().active.clone();
        if let Some(active) = active {
            active.cancel.cancel();
            wait_for(&active.done);
        }
    }
    pub fn begin_shutdown(&self) -> Option<Completion> {
        let mut inner = self.inner.lock().unwrap();
        // Use the import/publication mutex so no late request can start work
        // between capturing the active worker and stopping HTTP acceptance.
        inner.shutting_down = true;
        inner.active.as_ref().map(|active| {
            active.cancel.cancel();
            active.done.clone()
        })
    }
}

pub fn wait_for(completion: &Completion) {
    let (lock, done) = &**completion;
    let mut complete = lock.lock().unwrap();
    while !*complete {
        complete = done.wait(complete).unwrap();
    }
}
