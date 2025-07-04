pub mod database;
pub mod error;
pub mod file_dialog;
pub mod library;
pub mod node;

use crate::{
    database::Database,
    error::{CoreError, core_error},
    library::{Library, LibraryCommand, LibraryModel},
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
    node: NodeModel,
    library: LibraryModel,
}

/// Foreign trait implemented in Compose for receiving events from the Rust core.
#[uniffi::export(with_foreign)]
pub trait EventHandler: Send + Sync {
    fn on_update(&self, model: Model);
}

/// Long-lived object created by Compose as the entry point to the Rust core.
#[derive(uniffi::Object)]
pub struct Core {
    event_handler: Arc<dyn EventHandler>,
    node_tx: mpsc::UnboundedSender<NodeCommand>,
    library_tx: mpsc::UnboundedSender<LibraryCommand>,
}

#[uniffi::export]
impl Core {
    #[uniffi::constructor]
    pub fn new(event_handler: Arc<dyn EventHandler>) -> Result<Arc<Self>, CoreError> {
        #[cfg(target_os = "android")]
        {
            android_logger::init_once(
                android_logger::Config::default()
                    .with_max_level(log::LevelFilter::Trace) // limit log level
                    .with_tag("musicopy")
                    .with_filter(
                        android_logger::FilterBuilder::new()
                            .parse("debug,iroh=warn")
                            .build(),
                    ),
            );
        }
        #[cfg(not(target_os = "android"))]
        {
            env_logger::Builder::from_env(
                env_logger::Env::default().default_filter_or("debug,iroh=warn"),
            )
            .init();
        }
        log_panics::init();

        debug!("core: starting core");

        // TODO: pass arg for android dir
        let (db, secret_key) = if cfg!(target_os = "android") {
            let db = Database::open_in_memory().context("failed to open database")?;

            let secret_key = SecretKey::generate(rand::rngs::OsRng);

            (db, secret_key)
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

            (db, secret_key)
        };
        let db = Arc::new(Mutex::new(db));

        let node_id = NodeId::from(secret_key.public());

        let (node_tx, node_rx) = mpsc::unbounded_channel();
        let (library_tx, library_rx) = mpsc::unbounded_channel();

        // spawn node thread
        std::thread::spawn({
            let event_handler = event_handler.clone();
            move || {
                let builder = tokio::runtime::Builder::new_current_thread()
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
                    let library = Library::new(db, node_id).await.unwrap();
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
            node_tx,
            library_tx,
        }))
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

    pub fn rescan_library(&self) -> Result<(), CoreError> {
        self.library_tx
            .send(LibraryCommand::Rescan)
            .context("failed to send to library thread")?;
        Ok(())
    }
}
