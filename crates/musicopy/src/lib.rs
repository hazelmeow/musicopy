pub mod database;
pub mod error;
pub mod file_dialog;
pub mod fs;
pub mod library;
pub mod node;

use crate::{
    database::Database,
    error::{CoreError, core_error},
    library::{Library, LibraryCommand, LibraryModel, transcode::TranscodeStatusCache},
    node::{Node, NodeCommand, NodeModel},
};
use anyhow::Context;
use iroh::{NodeAddr, NodeId, SecretKey};
use log::{debug, error};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::mpsc;

uniffi::setup_scaffolding!();

/// State sent to Compose.
#[derive(Debug, uniffi::Record)]
pub struct Model {
    pub node: NodeModel,
    pub library: LibraryModel,
}

/// Foreign trait implemented in Compose for receiving events from the Rust core.
#[uniffi::export(with_foreign)]
pub trait EventHandler: Send + Sync {
    fn on_update(&self, model: Model);
}

#[derive(Debug, uniffi::Record)]
pub struct CoreOptions {
    pub init_logging: bool,
    pub in_memory: bool,
}

/// Long-lived object created by Compose as the entry point to the Rust core.
#[derive(uniffi::Object)]
pub struct Core {
    event_handler: Arc<dyn EventHandler>,

    db: Arc<Mutex<Database>>,

    node_tx: mpsc::UnboundedSender<NodeCommand>,
    library_tx: mpsc::UnboundedSender<LibraryCommand>,
}

// Stub debug implementation
impl std::fmt::Debug for Core {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Core").finish()
    }
}

#[uniffi::export]
impl Core {
    #[uniffi::constructor]
    pub fn new(
        event_handler: Arc<dyn EventHandler>,
        options: CoreOptions,
    ) -> Result<Arc<Self>, CoreError> {
        if options.init_logging {
            #[cfg(target_os = "android")]
            {
                android_logger::init_once(
                    android_logger::Config::default()
                        .with_max_level(log::LevelFilter::Trace) // limit log level
                        .with_tag("musicopy")
                        .with_filter(
                            android_logger::FilterBuilder::new()
                                .try_parse("musicopy=debug")
                                .expect("failed to parse log filter")
                                .build(),
                        ),
                );
            }
            #[cfg(not(target_os = "android"))]
            {
                env_logger::Builder::from_env(
                    env_logger::Env::default().default_filter_or("musicopy=debug"),
                )
                .init();
            }
            log_panics::init();
        }

        debug!("core: starting core");

        // TODO: pass arg for android dir
        let in_memory = cfg!(target_os = "android") || options.in_memory;

        let (db, secret_key, transcodes_dir) = if in_memory {
            let db = Database::open_in_memory().context("failed to open database")?;

            let secret_key = SecretKey::generate(rand::rngs::OsRng);

            // TODO: maybe clean contents or name uniquely each run
            let transcodes_dir = {
                // TODO: this doesn't work on android, but we only use it for transcoding for now
                let mut p = std::env::temp_dir();
                p.push("musicopy/transcodes");
                p
            };

            (db, secret_key, transcodes_dir)
        } else {
            let project_dirs = directories_next::ProjectDirs::from("", "", "musicopy")
                .context("failed to get project directories")?;
            let data_dir = project_dirs.data_local_dir();

            std::fs::create_dir_all(data_dir).context("failed to create data directory")?;

            let db = Database::open_file(&data_dir.join("musicopy.db"))
                .context("failed to open database")?;

            let key_path = data_dir.join("secret_key");
            let secret_key = if key_path.exists() {
                let key_bytes =
                    std::fs::read(&key_path).context("failed to read secret key file")?;
                SecretKey::from_bytes(
                    key_bytes[..32]
                        .try_into()
                        .context("failed to parse secret key file")?,
                )
            } else {
                let new_key = SecretKey::generate(rand::rngs::OsRng);
                std::fs::write(&key_path, new_key.to_bytes())
                    .context("failed to write secret key file")?;
                new_key
            };

            let transcodes_dir = {
                let cache_dir = project_dirs.cache_dir();
                cache_dir.join("transcodes")
            };

            (db, secret_key, transcodes_dir)
        };
        let db = Arc::new(Mutex::new(db));

        let transcode_status_cache = TranscodeStatusCache::new();

        let node_id = NodeId::from(secret_key.public());

        let (node_tx, node_rx) = mpsc::unbounded_channel();
        let (library_tx, library_rx) = mpsc::unbounded_channel();

        // spawn node thread
        std::thread::spawn({
            let event_handler = event_handler.clone();
            let db = db.clone();
            move || {
                // TODO: tune number of threads on mobile?
                let builder = tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .expect("should build runtime");

                builder.block_on(async move {
                    debug!("core: inside async runtime");

                    let db2 = db.clone();
                    let (node, run_token) = match Node::new(secret_key, db2).await {
                        Ok(x) => x,
                        Err(e) => {
                            error!("core: error creating node: {e:#}");
                            return;
                        }
                    };

                    debug!("core: inside async runtime - created node");

                    // TODO clean this up
                    // TODO pass path to files from app
                    let library = Library::new(db, node_id, transcodes_dir, transcode_status_cache)
                        .await
                        .unwrap();
                    tokio::spawn({
                        let library = library.clone();
                        async move {
                            if let Err(e) = library.run(library_rx).await {
                                error!("core: error running library: {e:#}");
                            }
                        }
                    });

                    // spawn state polling task
                    // TODO: reactive instead of polling?
                    tokio::spawn({
                        let node = node.clone();
                        async move {
                            debug!("core: inside polling task");

                            loop {
                                event_handler.on_update(Model {
                                    node: node.model(),
                                    library: library.model(),
                                });

                                tokio::time::sleep(std::time::Duration::from_secs_f64(1.0)).await;
                            }
                        }
                    });

                    debug!("core: inside async runtime - about to run node");

                    if let Err(e) = node.run(node_rx, run_token).await {
                        error!("core: error running node: {e:#}");
                    }

                    debug!("core: inside async runtime - exiting");
                });
            }
        });

        Ok(Arc::new(Self {
            event_handler,

            db,

            node_tx,
            library_tx,
        }))
    }

    pub fn shutdown(&self) -> Result<(), CoreError> {
        debug!("core: shutting down");

        self.node_tx
            .send(NodeCommand::Stop)
            .context("failed to send stop command to node thread")?;

        self.library_tx
            .send(LibraryCommand::Stop)
            .context("failed to send stop command to library thread")?;

        Ok(())
    }

    pub async fn connect(&self, node_id: &str) -> Result<(), CoreError> {
        let node_id: NodeId = node_id.parse().context("failed to parse node id")?;
        let node_addr = NodeAddr::from(node_id);

        let (callback_tx, callback_rx) = tokio::sync::oneshot::channel();

        self.node_tx
            .send(NodeCommand::Connect {
                addr: node_addr,
                callback: callback_tx,
            })
            .context("failed to send to node thread")?;

        async_std::future::timeout(Duration::from_secs(10), callback_rx)
            .await
            .map_err(|_elapsed| core_error!("connect timed out"))?
            .map_err(|_dropped| core_error!("connect failed, sender dropped"))?
            .map_err(CoreError::from)
    }

    pub fn download_all(&self, node_id: &str, download_directory: &str) -> Result<(), CoreError> {
        let node_id: NodeId = node_id.parse().context("failed to parse node id")?;

        // TODO: this shouldn't happen here
        self.node_tx
            .send(NodeCommand::SetDownloadDirectory(download_directory.into()))
            .context("failed to send to node thread")?;

        self.node_tx
            .send(NodeCommand::DownloadAll { client: node_id })
            .context("failed to send to node thread")?;

        Ok(())
    }

    pub fn accept_connection(&self, node_id: &str) -> Result<(), CoreError> {
        let node_id: NodeId = node_id.parse().context("failed to parse node id")?;

        self.node_tx
            .send(NodeCommand::AcceptConnection(node_id))
            .context("failed to send to node thread")?;

        Ok(())
    }

    pub fn deny_connection(&self, node_id: &str) -> Result<(), CoreError> {
        let node_id: NodeId = node_id.parse().context("failed to parse node id")?;

        self.node_tx
            .send(NodeCommand::DenyConnection(node_id))
            .context("failed to send to node thread")?;

        Ok(())
    }

    pub fn add_library_root(&self, name: String, path: String) -> Result<(), CoreError> {
        self.library_tx
            .send(LibraryCommand::AddRoot { name, path })
            .context("failed to send to library thread")?;

        Ok(())
    }

    pub fn remove_library_root(&self, name: String) -> Result<(), CoreError> {
        self.library_tx
            .send(LibraryCommand::RemoveRoot { name })
            .context("failed to send to library thread")?;

        Ok(())
    }

    // TODO: async wait for completion or return progress somehow
    pub fn rescan_library(&self) -> Result<(), CoreError> {
        self.library_tx
            .send(LibraryCommand::Rescan)
            .context("failed to send to library thread")?;
        Ok(())
    }

    pub fn reset_database(&self) -> Result<(), CoreError> {
        let db = self
            .db
            .lock()
            .map_err(|_elapsed| core_error!("failed to lock database"))?;

        db.reset()?;

        Ok(())
    }
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "system" fn Java_zip_meows_musicopy_RustNdkContext_init(
    env: jni::JNIEnv,
    _class: jni::objects::JClass,
    context: jni::objects::JObject,
) {
    let java_vm = env.get_java_vm().expect("failed to get java vm");
    let java_vm_ptr = java_vm.get_java_vm_pointer() as *mut std::ffi::c_void;

    // turn the local context reference into a global reference
    let context = env
        .new_global_ref(context)
        .expect("failed to create global ref for context");
    let context_ptr = context.as_raw() as *mut std::ffi::c_void;

    // leak the context global reference so it stays alive forever
    Box::leak(Box::new(context));

    unsafe {
        ndk_context::initialize_android_context(java_vm_ptr, context_ptr);
    }
}
