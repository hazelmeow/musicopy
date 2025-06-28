pub mod node;

use crate::node::{Node, NodeCommand, NodeModel};
use anyhow::Context;
use iroh::{NodeAddr, NodeId};
use log::{debug, error};
use std::sync::Arc;
use tokio::sync::mpsc;

uniffi::setup_scaffolding!();

/// Error type for FFI.
#[derive(Debug, thiserror::Error, uniffi::Object)]
#[error("{e:?}")]
pub struct CoreError {
    e: anyhow::Error,
}

#[uniffi::export]
impl CoreError {
    fn message(&self) -> String {
        self.to_string()
    }
}

impl From<anyhow::Error> for CoreError {
    fn from(e: anyhow::Error) -> Self {
        Self { e }
    }
}

/// State sent to Compose.
#[derive(Debug, uniffi::Record)]
pub struct Model {
    update_count: u32,
    node: Option<NodeModel>,
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
    tx: mpsc::UnboundedSender<NodeCommand>,
}

#[uniffi::export]
impl Core {
    #[uniffi::constructor]
    pub fn new(event_handler: Arc<dyn EventHandler>) -> Result<Arc<Self>, CoreError> {
        android_logger::init_once(
            android_logger::Config::default()
                .with_max_level(log::LevelFilter::Trace) // limit log level
                .with_tag("irohcompose")
                .with_filter(
                    android_logger::FilterBuilder::new()
                        .parse("debug,iroh=warn")
                        .build(),
                ),
        );

        debug!("core: starting core");

        let (tx, rx) = mpsc::unbounded_channel();

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

                    let node = match Node::new().await {
                        Ok(x) => x,
                        Err(e) => {
                            error!("core: error creating node: {e:#}");
                            return;
                        }
                    };

                    debug!("core: inside async runtime - created node");

                    // spawn state polling task
                    // TODO: reactive instead of polling?
                    tokio::spawn({
                        let node = node.clone();
                        async move {
                            debug!("core: inside polling task");

                            let mut update_count = 0;

                            loop {
                                update_count += 1;

                                event_handler.on_update(Model {
                                    update_count,
                                    node: Some(node.model()),
                                });

                                tokio::time::sleep(std::time::Duration::from_secs_f64(1.0)).await;
                            }
                        }
                    });

                    debug!("core: inside async runtime - about to run node");

                    if let Err(e) = node.run(rx).await {
                        error!("core: error running node: {e:#}");
                    }

                    debug!("core: inside async runtime - exiting");
                });
            }
        });

        Ok(Arc::new(Self { event_handler, tx }))
    }

    pub fn send(&self, node_id: &str, text: String) -> Result<(), CoreError> {
        let node_id: NodeId = node_id.parse().context("failed to parse node id")?;
        let node_addr = NodeAddr::from(node_id);

        self.tx
            .send(NodeCommand::Send(node_addr, text))
            .context("failed to send to node thread")?;

        Ok(())
    }
}
