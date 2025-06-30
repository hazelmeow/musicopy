pub mod error;
pub mod node;

use crate::{
    error::CoreError,
    node::{Node, NodeCommand, NodeModel},
};
use anyhow::Context;
use iroh::{NodeAddr, NodeId};
use log::{debug, error};
use std::sync::Arc;
use tokio::sync::mpsc;

uniffi::setup_scaffolding!();

/// State sent to Compose.
#[derive(Debug, uniffi::Record)]
pub struct Model {
    node: NodeModel,
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
                .with_tag("musicopy")
                .with_filter(
                    android_logger::FilterBuilder::new()
                        .parse("debug,iroh=warn")
                        .build(),
                ),
        );
        log_panics::init();

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

                    let (node, run_token) = match Node::new().await {
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

                            loop {
                                event_handler.on_update(Model { node: node.model() });

                                tokio::time::sleep(std::time::Duration::from_secs_f64(1.0)).await;
                            }
                        }
                    });

                    debug!("core: inside async runtime - about to run node");

                    if let Err(e) = node.run(rx, run_token).await {
                        error!("core: error running node: {e:#}");
                    }

                    debug!("core: inside async runtime - exiting");
                });
            }
        });

        Ok(Arc::new(Self { event_handler, tx }))
    }

    pub fn connect(&self, node_id: &str) -> Result<(), CoreError> {
        let node_id: NodeId = node_id.parse().context("failed to parse node id")?;
        let node_addr = NodeAddr::from(node_id);

        self.tx
            .send(NodeCommand::Connect(node_addr))
            .context("failed to send to node thread")?;

        Ok(())
    }

    pub fn accept_connection(&self, node_id: &str) -> Result<(), CoreError> {
        let node_id: NodeId = node_id.parse().context("failed to parse node id")?;

        self.tx
            .send(NodeCommand::AcceptConnection(node_id))
            .context("failed to send to node thread")?;

        Ok(())
    }
    pub fn deny_connection(&self, node_id: &str) -> Result<(), CoreError> {
        let node_id: NodeId = node_id.parse().context("failed to parse node id")?;

        self.tx
            .send(NodeCommand::DenyConnection(node_id))
            .context("failed to send to node thread")?;

        Ok(())
    }
}
