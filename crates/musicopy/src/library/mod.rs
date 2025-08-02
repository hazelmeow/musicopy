pub mod transcode;

use crate::{
    database::{Database, InsertFile},
    library::transcode::{TranscodeCommand, TranscodeItem, TranscodePool, TranscodeStatusCache},
    model::CounterModel,
};
use anyhow::Context;
use iroh::NodeId;
use itertools::Itertools;
use rayon::{iter::Either, prelude::*};
use std::{
    hash::Hasher,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use symphonia::core::{
    codecs::audio::VerificationCheck,
    formats::{FormatOptions, TrackType, probe::Hint},
    io::MediaSourceStream,
    meta::MetadataOptions,
};
use twox_hash::XxHash3_64;

#[derive(Debug, uniffi::Record)]
pub struct LibraryRootModel {
    pub name: String,
    pub path: String,
    pub num_files: u64,
}

#[derive(Debug, uniffi::Record)]
pub struct LibraryModel {
    pub local_roots: Vec<LibraryRootModel>,

    pub transcode_count_queued: Arc<CounterModel>,
    pub transcode_count_inprogress: Arc<CounterModel>,
    pub transcode_count_ready: Arc<CounterModel>,
    pub transcode_count_failed: Arc<CounterModel>,
}

#[derive(Debug)]
pub enum LibraryCommand {
    AddRoot { name: String, path: String },
    RemoveRoot { name: String },
    Rescan,

    Stop,
}

pub struct Library {
    db: Arc<Mutex<Database>>,
    local_node_id: NodeId,

    transcode_pool: TranscodePool,
}

impl Library {
    pub async fn new(
        db: Arc<Mutex<Database>>,
        local_node_id: NodeId,
        transcodes_dir: PathBuf,
        transcode_status_cache: TranscodeStatusCache,
    ) -> anyhow::Result<Arc<Self>> {
        // spawn transcode pool task
        let transcode_pool = TranscodePool::spawn(transcodes_dir, transcode_status_cache);

        let library = Arc::new(Self {
            db,
            local_node_id,
            transcode_pool,
        });

        // send all local files to the transcode pool to be transcoded if needed
        library
            .check_transcodes()
            .context("failed to check transcodes")?;

        Ok(library)
    }

    pub async fn run(
        self: &Arc<Self>,
        mut rx: tokio::sync::mpsc::UnboundedReceiver<LibraryCommand>,
    ) -> anyhow::Result<()> {
        loop {
            tokio::select! {
                Some(command) = rx.recv() => {
                    match command {
                        LibraryCommand::AddRoot { name, path } => {
                            {
                                let db = self.db.lock().unwrap();
                                let path = PathBuf::from(path);
                                let path = path.canonicalize().context("failed to canonicalize path")?;
                                db.add_root(self.local_node_id, &name, &path.to_string_lossy()).context("failed to add root")?;
                            }

                            // TODO
                            // self.notify_state();

                            // rescan the library after adding roots
                            self.spawn_scan();
                        }
                        LibraryCommand::RemoveRoot { name } => {
                            {
                                let db = self.db.lock().unwrap();
                                db.delete_root_by_name(self.local_node_id, &name).context("failed to delete root")?;
                            }

                            // TODO: remove files from root

                            // TODO
                            // self.notify_state();

                            // rescan the library after adding roots
                            self.spawn_scan();
                        }
                        LibraryCommand::Rescan => {
                            self.spawn_scan();
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
        let protocol = self.clone();
        tokio::spawn(async move {
            log::debug!("spawning library scan");
            if let Err(e) = protocol.scan().await {
                log::error!("error scanning library: {e:#}");
            }
            log::debug!("finished library scan");
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
                    Err(item) => Either::Right(item),
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

    pub fn model(&self) -> LibraryModel {
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

        LibraryModel {
            local_roots,
            
            transcode_count_queued: Arc::new(self.transcode_pool.queued_count_model()),
            transcode_count_inprogress: Arc::new(self.transcode_pool.inprogress_count_model()),
            transcode_count_ready: Arc::new(self.transcode_pool.ready_count_model()),
            transcode_count_failed: Arc::new(self.transcode_pool.failed_count_model()),
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
        hint.with_extension(extension.to_str().unwrap());
    }

    let meta_opts = MetadataOptions::default();
    let fmt_opts = FormatOptions::default();

    let mut probe = symphonia::default::get_probe()
        .probe(&hint, mss, fmt_opts, meta_opts)
        .context("failed to probe file")?;

    let audio_track = probe
        .default_track(TrackType::Audio)
        .context("failed to get default audio track")?;
    let audio_track_id = audio_track.id;

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
            let packet = match probe.next_packet() {
                Ok(Some(packet)) => packet,
                Ok(None) => break,
                Err(e) => anyhow::bail!("failed to read packet: {e}"),
            };
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
