pub mod transcode;

use crate::{
    EventHandler,
    database::{Database, InsertFile},
    library::transcode::{
        TranscodeCommand, TranscodeItem, TranscodePolicy, TranscodePool, TranscodeStatusCache,
    },
    model::CounterModel,
    node::FileSizeModel,
};
use anyhow::Context;
use iroh::NodeId;
use itertools::Itertools;
use log::warn;
use rayon::{iter::Either, prelude::*};
use std::{
    hash::Hasher,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};
use symphonia::core::{
    codecs::audio::VerificationCheck,
    formats::{TrackType, probe::Hint},
    io::MediaSourceStream,
};
use tokio::sync::mpsc;
use twox_hash::XxHash3_64;

#[derive(Debug, Clone, uniffi::Record)]
pub struct LibraryRootModel {
    pub name: String,
    pub path: String,
    pub num_files: u64,
}

/// Library state sent to the UI.
///
/// Needs to be Clone to send snapshots to the UI.
#[derive(Debug, Clone, uniffi::Record)]
pub struct LibraryModel {
    pub local_roots: Vec<LibraryRootModel>,

    pub transcodes_dir: String,
    pub transcodes_dir_size: FileSizeModel,

    pub transcode_count_queued: Arc<CounterModel>,
    pub transcode_count_inprogress: Arc<CounterModel>,
    pub transcode_count_ready: Arc<CounterModel>,
    pub transcode_count_failed: Arc<CounterModel>,

    pub transcode_policy: TranscodePolicy,
}

#[derive(Debug)]
pub enum LibraryCommand {
    AddRoot { name: String, path: String },
    RemoveRoot { name: String },
    Rescan,

    PrioritizeTranscodes(Vec<(String, Vec<u8>)>),
    SetTranscodePolicy(TranscodePolicy),

    Stop,
}

/// An update to the library model.
enum LibraryModelUpdate {
    UpdateLocalRoots,
    UpdateTranscodesDirSize,
    SetTranscodePolicy(TranscodePolicy),
}

pub struct Library {
    event_handler: Arc<dyn EventHandler>,
    db: Arc<Mutex<Database>>,
    local_node_id: NodeId,

    transcode_pool: TranscodePool,

    command_tx: mpsc::UnboundedSender<LibraryCommand>,

    model: Mutex<LibraryModel>,
}

// stub debug implementation
impl std::fmt::Debug for Library {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Library").finish()
    }
}

/// The resources needed to run the Library run loop.
///
/// This is created by Library::new() and passed linearly to Library::run().
/// This pattern allows the run loop to own and mutate these resources while
/// hiding the details from the public API.
#[derive(Debug)]
pub struct LibraryRun {
    command_rx: mpsc::UnboundedReceiver<LibraryCommand>,
}

impl Library {
    pub async fn new(
        event_handler: Arc<dyn EventHandler>,
        db: Arc<Mutex<Database>>,
        local_node_id: NodeId,
        transcodes_dir: PathBuf,
        transcode_policy: TranscodePolicy,
        transcode_status_cache: TranscodeStatusCache,
    ) -> anyhow::Result<(Arc<Self>, LibraryRun)> {
        // spawn transcode pool task
        let transcode_pool = TranscodePool::spawn(
            transcodes_dir.clone(),
            transcode_policy,
            transcode_status_cache,
        );

        let (command_tx, command_rx) = mpsc::unbounded_channel();

        let model = LibraryModel {
            local_roots: Vec::new(),

            transcodes_dir: transcode_pool.transcodes_dir(),
            transcodes_dir_size: transcode_pool.transcodes_dir_size(),

            transcode_count_queued: Arc::new(transcode_pool.queued_count_model()),
            transcode_count_inprogress: Arc::new(transcode_pool.inprogress_count_model()),
            transcode_count_ready: Arc::new(transcode_pool.ready_count_model()),
            transcode_count_failed: Arc::new(transcode_pool.failed_count_model()),

            transcode_policy,
        };

        let library = Arc::new(Self {
            event_handler,
            db,
            local_node_id,

            transcode_pool,

            command_tx,

            model: Mutex::new(model),
        });

        // initialize model
        // TODO: don't push updates during init
        library.update_model(LibraryModelUpdate::UpdateLocalRoots);

        // send all local files to the transcode pool to be transcoded if needed
        library
            .check_transcodes()
            .context("failed to check transcodes")?;

        // spawn transcodes dir size polling task
        tokio::spawn({
            let library = library.clone();
            async move {
                loop {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    library.update_model(LibraryModelUpdate::UpdateTranscodesDirSize);
                }
            }
        });

        let library_run = LibraryRun { command_rx };

        Ok((library, library_run))
    }

    pub async fn run(self: &Arc<Self>, run_token: LibraryRun) -> anyhow::Result<()> {
        let LibraryRun { mut command_rx } = run_token;

        loop {
            tokio::select! {
                Some(command) = command_rx.recv() => {
                    match command {
                        LibraryCommand::AddRoot { name, path } => {
                            {
                                let db = self.db.lock().unwrap();
                                let path = PathBuf::from(path);
                                let path = path.canonicalize().context("failed to canonicalize path")?;
                                db.add_root(self.local_node_id, &name, &path.to_string_lossy()).context("failed to add root")?;
                            }

                            // update model
                            self.update_model(LibraryModelUpdate::UpdateLocalRoots);

                            // rescan the library
                            self.spawn_scan();
                        }

                        LibraryCommand::RemoveRoot { name } => {
                            {
                                let db = self.db.lock().unwrap();
                                db.delete_root_by_name(self.local_node_id, &name).context("failed to delete root")?;
                            }

                            // TODO: remove files from root

                            // update model
                            self.update_model(LibraryModelUpdate::UpdateLocalRoots);

                            // rescan the library
                            self.spawn_scan();
                        }

                        LibraryCommand::Rescan => {
                            self.spawn_scan();
                        }

                        LibraryCommand::PrioritizeTranscodes(hashes) => {
                            if let Err(e) = self.transcode_pool.send(TranscodeCommand::Prioritize(hashes.clone())) {
                                warn!("LibraryCommand::PrioritizeTranscodes: failed to send to transcode pool: {e:#}");
                            }
                        }

                        LibraryCommand::SetTranscodePolicy(transcode_policy) => {
                            if let Err(e) = self.transcode_pool.send(TranscodeCommand::SetPolicy(transcode_policy)) {
                                warn!("LibraryCommand::SetTranscodePolicy: failed to send to transcode pool: {e:#}");
                            }

                            // update model
                            self.update_model(LibraryModelUpdate::SetTranscodePolicy(transcode_policy));
                        }

                        LibraryCommand::Stop => {
                            break;
                        }
                    }
                }

                else => {
                    log::warn!("all senders dropped in Library::run, shutting down");
                    break;
                }
            }
        }

        Ok(())
    }

    fn spawn_scan(self: &Arc<Self>) {
        let library = self.clone();
        tokio::spawn(async move {
            log::debug!("spawning library scan");
            if let Err(e) = library.scan().await {
                log::error!("error scanning library: {e:#}");
            }
            log::debug!("finished library scan");

            // update root file counts in model
            library.update_model(LibraryModelUpdate::UpdateLocalRoots);
        });
    }

    // TODO: lock so only one scan is running at a time
    // TODO: check perf when scanning large libraries
    // TODO: incremental scans? using existing files to skip work, instead of redoing everything? maybe only when new roots added?
    async fn scan(self: &Arc<Self>) -> anyhow::Result<()> {
        let mut errors = Vec::new();

        let (roots, prev_local_files) = {
            let db = self.db.lock().unwrap();
            let roots = db
                .get_roots_by_node_id(self.local_node_id)
                .context("failed to get local roots")?;
            // let local_files = db.get_local_files().context("failed to get local files")?;
            let local_files = (); // TODO
            (roots, local_files)
        };

        log::info!("scan: scanning {} roots", roots.len());

        // remove roots that don't exist
        let roots = roots
            .into_iter()
            .filter(|root| {
                let path = PathBuf::from(&root.path);
                if path.exists() {
                    true
                } else {
                    errors.push(anyhow::anyhow!(
                        "root path `{}` does not exist",
                        path.display()
                    ));
                    false
                }
            })
            .collect::<Vec<_>>();

        // walk roots and collect entries
        let (entries, walk_errors): (Vec<_>, Vec<_>) = roots
            .iter()
            .flat_map(|root| {
                let walker = globwalk::GlobWalkerBuilder::new(
                    &root.path,
                    "*.{mp3,flac,ogg,m4a,wav,aif,aiff}",
                )
                .file_type(globwalk::FileType::FILE)
                .build()
                .expect("glob shouldn't fail");

                walker.into_iter().map_ok(move |entry| (root, entry))
            })
            .partition_result();

        log::info!("scan: found {} files", entries.len());

        // extend errors
        errors.extend(
            walk_errors
                .into_iter()
                .map(|e| anyhow::anyhow!("failed to scan file {:?}: {}", e.path(), e)),
        );

        struct ScanItem {
            root: String,
            path: String,
            local_path: String,
        }

        let (local_files, scan_errors): (Vec<_>, Vec<_>) = entries
            .into_iter()
            .map(|(root, entry)| {
                let local_path = entry.into_path();

                // get path without root
                let path = local_path
                    .strip_prefix(&root.path)
                    .context("failed to strip root path prefix")?
                    .to_string_lossy()
                    .to_string();

                anyhow::Result::Ok(ScanItem {
                    root: root.name.clone(),
                    path,
                    local_path: local_path.to_string_lossy().to_string(),
                })
            })
            .partition_result();

        // extend errors
        errors.extend(
            scan_errors
                .into_iter()
                .map(|e: anyhow::Error| e.context("failed to scan file")),
        );

        struct HashItem {
            hash_kind: &'static str,
            hash: Vec<u8>,
            root: String,
            path: String,
            local_path: String,
        }

        // hash items in parallel using rayon
        let (items, hash_errors): (Vec<_>, Vec<_>) = tokio::task::spawn_blocking(move || {
            local_files
                .into_par_iter()
                .map(|item| {
                    let local_path = PathBuf::from(&item.local_path);

                    let (hash_kind, hash) = get_file_hash(&local_path)?;

                    anyhow::Result::Ok(HashItem {
                        hash_kind,
                        hash,
                        root: item.root,
                        path: item.path,
                        local_path: item.local_path,
                    })
                })
                .map(|res| match res {
                    Ok(item) => Either::Left(item),
                    Err(e) => Either::Right(e),
                })
                .collect::<(Vec<_>, Vec<_>)>()
        })
        .await?;

        // extend errors
        errors.extend(hash_errors);

        for error in errors {
            log::error!("error scanning library: {error:#}");
        }

        {
            let mut db = self.db.lock().unwrap();
            db.replace_local_files(
                self.local_node_id,
                items.iter().map(|item| InsertFile {
                    hash_kind: item.hash_kind,
                    hash: &item.hash,
                    root: &item.root,
                    path: &item.path,
                    local_path: &item.local_path,
                }),
            )
            .context("failed to insert files into database")?;
        }

        log::info!("scan: inserted {} files into database", items.len());

        // send local files to transcode pool
        // will be skipped if already transcoded. might be able to make more efficient by only sending new files
        // TODO: cancel transcodes for files that were removed
        {
            let transcode_add_items = items
                .into_iter()
                .map(|item| TranscodeItem {
                    hash_kind: item.hash_kind.to_string(),
                    hash: item.hash,
                    local_path: PathBuf::from(item.local_path),
                })
                .collect::<Vec<_>>();

            self.transcode_pool
                .send(TranscodeCommand::Add(transcode_add_items))?;
        }

        // TODO
        // self.notify_state();

        Ok(())
    }

    /// Send all local files to the transcode pool to be transcoded if needed.
    fn check_transcodes(&self) -> anyhow::Result<()> {
        let local_files = {
            let db = self.db.lock().expect("failed to lock database");
            db.get_files_by_node_id(self.local_node_id)
                .context("failed to get local files")?
        };

        let transcode_add_items = local_files
            .into_iter()
            .map(|file| TranscodeItem {
                hash_kind: file.hash_kind,
                hash: file.hash,
                local_path: PathBuf::from(file.local_path),
            })
            .collect::<Vec<_>>();

        self.transcode_pool
            .send(TranscodeCommand::Add(transcode_add_items))?;

        Ok(())
    }

    pub fn send(self: &Arc<Self>, command: LibraryCommand) -> anyhow::Result<()> {
        self.command_tx
            .send(command)
            .map_err(|e| anyhow::anyhow!("failed to send command: {e:?}"))
    }

    pub fn get_model(self: &Arc<Self>) -> LibraryModel {
        let model = self.model.lock().unwrap();
        model.clone()
    }

    // TODO: throttle pushing updates?
    fn update_model(self: &Arc<Self>, update: LibraryModelUpdate) {
        match update {
            LibraryModelUpdate::UpdateLocalRoots => {
                let local_roots = {
                    let db = self.db.lock().unwrap();
                    db.get_roots_by_node_id(self.local_node_id)
                        .expect("failed to get local roots")
                        .into_iter()
                        .map(|root| {
                            let count = db
                                .count_files_by_root(self.local_node_id, &root.name)
                                .expect("failed to count files"); // TODO

                            LibraryRootModel {
                                name: root.name,
                                path: root.path,
                                num_files: count,
                            }
                        })
                        .collect()
                };

                let mut model = self.model.lock().unwrap();
                model.local_roots = local_roots;

                self.event_handler.on_library_model_snapshot(model.clone());
            }

            LibraryModelUpdate::UpdateTranscodesDirSize => {
                let mut model = self.model.lock().unwrap();
                model.transcodes_dir_size = self.transcode_pool.transcodes_dir_size();

                self.event_handler.on_library_model_snapshot(model.clone());
            }

            LibraryModelUpdate::SetTranscodePolicy(transcode_policy) => {
                let mut model = self.model.lock().unwrap();
                model.transcode_policy = transcode_policy;

                self.event_handler.on_library_model_snapshot(model.clone());
            }
        }
    }
}

/// Get the hash of a file.
///
/// If the file contains an MD5 checksum (many flacs do), then it will be used.
/// Otherwise, the file will be decoded and the audio data will be hashed using
/// xxhash3 with 64-bit hashes.
fn get_file_hash(path: &PathBuf) -> anyhow::Result<(&'static str, Vec<u8>)> {
    let src = std::fs::File::open(path).context("failed to open file")?;

    let mss = MediaSourceStream::new(Box::new(src), Default::default());

    let mut hint = Hint::new();
    if let Some(extension) = path.extension() {
        hint.with_extension(extension.to_str().context("invalid file extension")?);
    }

    let mut format = symphonia::default::get_probe()
        .probe(&hint, mss, Default::default(), Default::default())
        .context("failed to probe file")?;

    // get the default audio track
    let audio_track = format
        .default_track(TrackType::Audio)
        .context("failed to get default audio track")?;
    let audio_track_id = audio_track.id;

    // check if MD5 verification check is available (common for flacs)
    if let Some(VerificationCheck::Md5(verification_md5)) = &audio_track
        .codec_params
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("failed to get codec params"))?
        .audio()
        .ok_or_else(|| anyhow::anyhow!("failed to get audio codec params"))?
        .verification_check
    {
        Ok(("md5", Vec::from(verification_md5)))
    } else {
        let mut hasher = XxHash3_64::with_seed(8888);

        loop {
            // read next packet
            let packet = match format.next_packet() {
                Ok(Some(packet)) => packet,

                // end of track
                Ok(None) => break,

                Err(e) => anyhow::bail!("failed to read packet: {e}"),
            };

            // skip packets from other tracks
            if packet.track_id() != audio_track_id {
                continue;
            }

            // hash the packet bytes, without decoding them.
            // this is maybe more stable than hashing the decoded samples, and
            // should still stay the same when metadata is modified.
            hasher.write(packet.buf());
        }

        // the convention for xxhash is to use big-endian byte order
        // https://github.com/Cyan4973/xxHash/blob/55d9c43608e39b2acd7d9a9cc3df424f812b6642/xxhash.h#L192
        let hash = hasher.finish().to_be_bytes();

        Ok(("xxh3", Vec::from(hash)))
    }
}
