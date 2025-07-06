use crate::database::{Database, InsertFile};
use anyhow::Context;
use iroh::NodeId;
use itertools::Itertools;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

#[derive(Debug, uniffi::Record)]
pub struct LibraryRootModel {
    pub name: String,
    pub path: String,
    pub num_files: u64,
}

#[derive(Debug, uniffi::Record)]
pub struct LibraryModel {
    pub local_roots: Vec<LibraryRootModel>,
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
}

impl Library {
    pub async fn new(db: Arc<Mutex<Database>>, local_node_id: NodeId) -> anyhow::Result<Arc<Self>> {
        let library = Arc::new(Self { db, local_node_id });

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
                println!("error scanning library: {e:#}");
            }
            log::debug!("finished library scan");
        });
    }

    // TODO: stream results asynchronously? scanning the fs is fast but transcoding is slow,
    // so when do we do that?
    async fn scan(self: &Arc<Self>) -> anyhow::Result<()> {
        // TODO: lock so only one scan is running at a time

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
                    // hash_kind: "sha256".to_string(),
                    // hash: "".to_string(),
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

        for error in errors {
            log::error!("error scanning library: {error:#}");
        }

        {
            let mut db = self.db.lock().unwrap();
            db.insert_files(local_files.iter().map(|item| InsertFile {
                hash_kind: "sha256",
                hash: "",
                node_id: self.local_node_id,
                root: &item.root,
                path: &item.path,
                local_path: &item.local_path,
            }))?;
        }

        log::info!("scan: inserted {} files into database", local_files.len());

        let index = local_files
            .iter()
            .map(|item| {
                // TODO
                let full_path = format!("{}/{}", item.root, item.path);
                full_path
            })
            .collect::<Vec<String>>();

        // TODO
        // self.library.store(Arc::new(index));

        // TODO
        // self.notify_state();

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

        LibraryModel { local_roots }
    }
}
