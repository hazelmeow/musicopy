use anyhow::Context;
use dashmap::DashMap;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio::sync::mpsc;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey(String, Vec<u8>);

/// An in-memory cache of the transcoding status of files.
///
/// This is populated on startup by reading the transcode cache directory and
/// updated as files are transcoded. It's initialized in Core and passed down
/// to TranscodePool because it needs to be shared with Node as well.
///
/// We need to key by hash because rescanning causes file IDs to change and
/// can happen at any time. This also accounts for multiple copies of the same
/// file existing in the library.
#[derive(Debug, Clone)]
pub struct TranscodeStatusCache(Arc<DashMap<CacheKey, TranscodeStatus>>);

impl TranscodeStatusCache {
    /// Create a new TranscodeStatusCache.
    pub fn new() -> Self {
        TranscodeStatusCache(Arc::new(DashMap::new()))
    }
}

impl Default for TranscodeStatusCache {
    fn default() -> Self {
        Self::new()
    }
}

/// An item in the transcoding queue.
pub struct TranscodeItem {
    pub hash_kind: String,
    pub hash: Vec<u8>,
    pub local_path: PathBuf,
}

/// A command sent to the transcoding pool.
pub enum TranscodeCommand {
    /// Sent when files are added to the library. Files are enqueued if they
    /// aren't already transcoded or in the queue.
    ///
    /// It's inefficient to send files that are already transcoded, but it is
    /// safe to do so, and they will not be transcoded again.
    Add(Vec<TranscodeItem>),

    /// Increase the priority of some files. Sent when files are requested.
    /// This is useful for partial downloads when the library isn't fully
    /// transcoded yet.
    Prioritize(Vec<u64>),

    /// Sent when files are removed from the library. Files are dequeued if
    /// they are currently queued for transcoding.
    Remove(Vec<TranscodeItem>),

    /// Delete transcodes of files that aren't in the library anymore.
    CollectGarbage(Vec<u64>),
}

/// The transcode status of a file.
#[derive(Debug)]
pub enum TranscodeStatus {
    Queued,
    Done(PathBuf),
}

/// A pool of worker threads for transcoding files.
pub struct TranscodePool {
    transcodes_dir: PathBuf,
    status_cache: TranscodeStatusCache,
}

impl TranscodePool {
    /// Create a TranscodePool.
    ///
    /// The transcode status cache is guaranteed to be populated after this
    /// returns.
    pub fn new(transcodes_dir: PathBuf, status_cache: TranscodeStatusCache) -> Self {
        let pool = TranscodePool {
            transcodes_dir,
            status_cache,
        };

        pool.read_transcodes_dir();

        pool
    }

    // initialize the transcode status cache by reading the transcode cache directory
    fn read_transcodes_dir(&self) {
        // create transcode cache directory if it doesn't exist
        if let Err(e) = std::fs::create_dir_all(&self.transcodes_dir) {
            log::error!(
                "failed to create transcode cache directory at {}: {}",
                self.transcodes_dir.display(),
                e
            );
        }

        // read the transcode cache directory
        let items = match std::fs::read_dir(&self.transcodes_dir) {
            Ok(entries) => entries,
            Err(e) => {
                log::error!(
                    "failed to read transcode cache directory at {}: {}",
                    self.transcodes_dir.display(),
                    e
                );
                return;
            }
        };

        // parse transcode cache directory entries
        let items = items
            .filter_map(|entry| match entry {
                Ok(entry) => Some(entry),
                Err(e) => {
                    log::error!("failed to read entry in transcode cache directory: {}", e);
                    None
                }
            })
            .filter_map(|entry| match self.parse_transcodes_dir_entry(&entry) {
                Ok(res) => Some(res),
                Err(e) => {
                    log::error!(
                        "failed to parse transcode cache directory entry at {}: {}",
                        entry.path().display(),
                        e
                    );
                    None
                }
            })
            .collect::<Vec<_>>();

        // update status cache
        for (path, hash_kind, hash) in items {
            self.status_cache
                .0
                .insert(CacheKey(hash_kind, hash), TranscodeStatus::Done(path));
        }
    }

    fn parse_transcodes_dir_entry(
        &self,
        entry: &std::fs::DirEntry,
    ) -> anyhow::Result<(PathBuf, String, Vec<u8>)> {
        // get entry file type
        let file_type = entry.file_type().context("failed to get file type")?;

        // skip non-files
        if !file_type.is_file() {
            anyhow::bail!("entry is not a file");
        }

        // parse file name as <hash kind>-<hash hex>.ext
        let path = entry.path();
        let file_stem = path
            .file_stem()
            .context("file missing name")?
            .to_string_lossy();
        let (hash_kind, hash) = file_stem
            .split_once("-")
            .context("failed to parse file name")?;
        let hash_kind = hash_kind.to_string();
        let hash = hex::decode(hash).context("failed to decode hash bytes")?;

        Ok((path, hash_kind, hash))
    }

    pub async fn run(
        self,
        mut rx: mpsc::UnboundedReceiver<TranscodeCommand>,
    ) -> anyhow::Result<()> {
        let (job_tx, job_rx) = mpsc::unbounded_channel();
        let job_rx = Arc::new(Mutex::new(job_rx));

        for _ in 0..4 {
            TranscodeWorker::new(job_rx.clone());
        }

        loop {
            tokio::select! {
                Some(command) = rx.recv() => {
                    match command {
                        TranscodeCommand::Add(transcode_add_items) => {
                            for item in transcode_add_items {
                                // TODO: remove these clones
                                let key = CacheKey(item.hash_kind.clone(), item.hash.clone());
                                let status = self.status_cache.0.get(&key);
                                if status.is_none() {
                                    log::trace!("TranscodePool: queueing file {}", item.local_path.display());

                                    // set status to queued
                                    self.status_cache.0.insert(
                                        key,
                                        TranscodeStatus::Queued,
                                    );

                                    // send job to a worker
                                    job_tx.send(item).context("failed to send transcode job")?;
                                } else {
                                    log::trace!("TranscodePool: skipping file {}", item.local_path.display());
                                }
                            }
                        },
                        TranscodeCommand::Prioritize(items) => {
                            // TODO: we need to switch to a priority queue + 0-length bounded channel + dispatcher task
                            todo!()
                        },
                        TranscodeCommand::Remove(items) => {
                            // TODO: this also needs the pqueue because we can't remove the mpsc internal buffer
                            todo!()
                        },
                        TranscodeCommand::CollectGarbage(items) => todo!(),
                    }
                }
            }
        }
    }
}

struct TranscodeWorker {}

impl TranscodeWorker {
    /// Start a new transcode worker thread and return a handle to it.
    pub fn new(job_rx: Arc<Mutex<mpsc::UnboundedReceiver<TranscodeItem>>>) -> Self {
        std::thread::spawn(move || {
            if let Err(e) = Self::run(job_rx) {
                log::error!("transcode worker failed: {e:#}");
            }
        });

        Self {}
    }

    /// Implementation of the transcode worker thread.
    fn run(job_rx: Arc<Mutex<mpsc::UnboundedReceiver<TranscodeItem>>>) -> anyhow::Result<()> {
        loop {
            let job = {
                let mut job_rx = job_rx.lock().expect("failed to lock job receiver");
                let Some(job) = job_rx.blocking_recv() else {
                    log::warn!("transcode worker receiver closed, shutting down");
                    break;
                };
                job
            };

            // TODO
            log::info!("transcoding file: {:?}", job.local_path);
            std::thread::sleep(std::time::Duration::from_secs(3));
            log::info!("finished transcoding file: {:?}", job.local_path);
        }

        // worker shut down
        Ok(())
    }
}
