//! Networking.
//!
//! A `Node` is an Iroh node that can perform the client or server end of the protocol.
//!
//! A `Client` is an outgoing connection to a server.
//! Clients request files, primarily used in the mobile app.
//!
//! A `Server` is an incoming connection from a client.
//! Servers send files, primarily used in the desktop app.

use crate::{
    database::Database,
    fs::{OpenMode, TreeFile, TreePath},
    library::transcode::{TranscodeStatus, TranscodeStatusCache},
};
use anyhow::Context;
use dashmap::DashMap;
use futures::{SinkExt, StreamExt, TryStreamExt};
use iroh::{
    Endpoint, NodeAddr, NodeId, SecretKey,
    endpoint::Connection,
    protocol::{ProtocolHandler, Router},
};
use n0_future::future::Boxed;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::PathBuf,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, SystemTime},
};
use tokio::{
    io::{AsyncReadExt, AsyncWrite, AsyncWriteExt},
    sync::{mpsc, oneshot},
};
use tokio_util::{
    bytes::Bytes,
    codec::{FramedRead, FramedWrite, LengthDelimitedCodec},
};

#[derive(Debug, uniffi::Object)]
pub struct ProgressCounterModel(Arc<AtomicU64>);

#[uniffi::export]
impl ProgressCounterModel {
    #[uniffi::constructor]
    pub fn new(n: u64) -> Self {
        Self(Arc::new(AtomicU64::new(n)))
    }

    pub fn get(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}

/// Model of progress for a transfer job.
#[derive(Debug, uniffi::Enum)]
pub enum TransferJobProgressModel {
    Requested,
    Transcoding,
    Ready,
    InProgress {
        started_at: u64,
        /// Number of bytes written so far.
        bytes: Arc<ProgressCounterModel>,
    },
    Finished {
        finished_at: u64,
    },
    Failed {
        error: String,
    },
}

/// Model of a transfer job.
#[derive(Debug, uniffi::Record)]
pub struct TransferJobModel {
    pub job_id: u64,
    pub file_root: String,
    pub file_path: String,
    pub file_size: Option<u64>,
    pub progress: TransferJobProgressModel,
}

/// Model of an incoming connection.
#[derive(Debug, uniffi::Record)]
pub struct ServerModel {
    pub name: String,
    pub node_id: String,
    pub connected_at: u64,

    pub accepted: bool,

    pub connection_type: String,
    pub latency_ms: Option<u64>,

    pub transfer_jobs: Vec<TransferJobModel>,
}

/// Model of an item in the index sent by the server.
#[derive(Debug, uniffi::Record)]
pub struct IndexItemModel {
    pub node_id: String,
    pub hash_kind: String,
    pub hash: Vec<u8>,
    pub root: String,
    pub path: String,
}

/// Model of an outgoing connection.
#[derive(Debug, uniffi::Record)]
pub struct ClientModel {
    pub name: String,
    pub node_id: String,
    pub connected_at: u64,

    pub accepted: bool,

    pub connection_type: String,
    pub latency_ms: Option<u64>,

    pub index: Option<Vec<IndexItemModel>>,
    pub transfer_jobs: Vec<TransferJobModel>,
}

/// Node state sent to Compose.
#[derive(Debug, uniffi::Record)]
pub struct NodeModel {
    pub node_id: String,
    pub home_relay: String,

    pub send_ipv4: u64,
    pub send_ipv6: u64,
    pub send_relay: u64,
    pub recv_ipv4: u64,
    pub recv_ipv6: u64,
    pub recv_relay: u64,
    pub conn_success: u64,
    pub conn_direct: u64,

    pub servers: Vec<ServerModel>,
    pub clients: Vec<ClientModel>,
}

#[derive(Debug)]
pub enum NodeCommand {
    SetDownloadDirectory(String),

    Connect {
        addr: NodeAddr,
        callback: oneshot::Sender<anyhow::Result<()>>,
    },

    AcceptConnection(NodeId),
    DenyConnection(NodeId),

    DownloadAll {
        client: NodeId,
    },

    Stop,
}

#[derive(Debug)]
pub struct Node {
    db: Arc<Mutex<Database>>,
    transcode_status_cache: TranscodeStatusCache,

    router: Router,
    protocol: Protocol,

    client_handle_tx: mpsc::UnboundedSender<(NodeId, ClientHandle)>,

    servers: Mutex<HashMap<NodeId, ServerHandle>>,
    clients: Mutex<HashMap<NodeId, ClientHandle>>,

    download_directory: Arc<Mutex<Option<String>>>,
}

#[derive(Debug)]
pub struct NodeRunToken {
    server_handle_rx: mpsc::UnboundedReceiver<(NodeId, ServerHandle)>,
    server_closed_rx: mpsc::UnboundedReceiver<NodeId>,
    client_handle_rx: mpsc::UnboundedReceiver<(NodeId, ClientHandle)>,
}

impl Node {
    pub async fn new(
        secret_key: SecretKey,
        db: Arc<Mutex<Database>>,
        transcode_status_cache: TranscodeStatusCache,
    ) -> anyhow::Result<(Arc<Self>, NodeRunToken)> {
        let (server_handle_tx, server_handle_rx) = mpsc::unbounded_channel();
        let (server_closed_tx, server_closed_rx) = mpsc::unbounded_channel();
        let (client_handle_tx, client_handle_rx) = mpsc::unbounded_channel();

        let endpoint = Endpoint::builder()
            .secret_key(secret_key)
            .discovery_n0()
            .bind()
            .await?;
        let protocol = Protocol::new(
            db.clone(),
            transcode_status_cache.clone(),
            server_handle_tx,
            server_closed_tx,
        );

        let router = Router::builder(endpoint)
            .accept(Protocol::ALPN, protocol.clone())
            .spawn();

        let node = Arc::new(Self {
            db,
            transcode_status_cache,

            router,
            protocol,

            client_handle_tx,

            servers: Mutex::new(HashMap::new()),
            clients: Mutex::new(HashMap::new()),

            download_directory: Arc::new(Mutex::new(None)),
        });

        let node_run = NodeRunToken {
            server_handle_rx,
            server_closed_rx,
            client_handle_rx,
        };

        Ok((node, node_run))
    }

    pub async fn run(
        self: &Arc<Self>,
        mut rx: mpsc::UnboundedReceiver<NodeCommand>,
        run_token: NodeRunToken,
    ) -> anyhow::Result<()> {
        let NodeRunToken {
            mut server_handle_rx,
            mut server_closed_rx,
            mut client_handle_rx,
        } = run_token;

        loop {
            tokio::select! {
                Some(command) = rx.recv() => {
                    match command {
                        NodeCommand::SetDownloadDirectory(path) => {
                            let mut download_directory = self.download_directory.lock().unwrap();
                            *download_directory = Some(path);
                        },

                        NodeCommand::Connect { addr, callback } => {
                            let node = self.clone();
                            tokio::task::spawn(async move {
                                log::debug!("starting connect");
                                let res = node.connect(addr).await;
                                log::debug!("connect result: {res:?}");
                                if let Err(e) = callback.send(res) {
                                    log::error!("failed to send res: {e:?}");
                                }
                            });
                        },

                        NodeCommand::AcceptConnection(node_id) => {
                            let servers = self.servers.lock().unwrap();
                            if let Some(server_handle) = servers.get(&node_id) {
                                server_handle.tx.send(ServerCommand::Accept).expect("failed to send ServerCommand::Accept");
                            } else {
                                log::error!("AcceptConnection: no server found with node_id: {node_id}");
                            }
                        },
                        NodeCommand::DenyConnection(node_id) => {
                            let servers = self.servers.lock().unwrap();
                            if let Some(server_handle) = servers.get(&node_id) {
                                server_handle.tx.send(ServerCommand::Close).expect("failed to send ServerCommand::Close");
                            } else {
                                log::error!("DenyConnection: no server found with node_id: {node_id}");
                            }
                        },

                        NodeCommand::DownloadAll { client } => {
                            // check that download directory is set before downloading
                            {
                                let download_directory = self.download_directory.lock().unwrap();
                                if download_directory.is_none() {
                                    log::error!("DownloadAll: download directory not set");
                                    continue;
                                }
                            };

                            let clients = self.clients.lock().unwrap();
                            if let Some(client_handle) = clients.get(&client) {
                                client_handle.tx.send(ClientCommand::DownloadAll).expect("failed to send ClientCommand::DownloadAll");
                            } else {
                                log::error!("DownloadAll: no client found with node_id: {client}");
                            }
                        },

                        NodeCommand::Stop => break,
                    }
                }

                Some((server_id, server_handle)) = server_handle_rx.recv() => {
                    let mut servers = self.servers.lock().unwrap();
                    servers.insert(server_id, server_handle);
                }

                Some(server_id) = server_closed_rx.recv() => {
                    let mut servers = self.servers.lock().unwrap();
                    servers.remove(&server_id);
                }

                Some((client_id, client_handle)) = client_handle_rx.recv() => {
                    let mut clients = self.clients.lock().unwrap();
                    clients.insert(client_id, client_handle);
                }

                else => {
                    log::warn!("all senders dropped in Node::run, shutting down");
                    break
                }
            }
        }

        let _ = self.router.shutdown().await;

        Ok(())
    }

    async fn connect(self: &Arc<Self>, addr: NodeAddr) -> anyhow::Result<()> {
        // connect before spawning the task, so we can return an error immediately
        let connection = self.router.endpoint().connect(addr, Protocol::ALPN).await?;

        let node_id = connection.remote_node_id()?;
        log::info!("opened connection to {node_id}");

        let client_handle_tx = self.client_handle_tx.clone();
        let node = self.clone();
        let download_directory = self.download_directory.clone();
        tokio::spawn(async move {
            let client = Client::new(client_handle_tx, connection, download_directory);

            if let Err(e) = client.run().await {
                log::error!("error during client.run(): {e:#}");
            }

            // remove handle from hashmap
            {
                let mut clients = node.clients.lock().unwrap();
                clients.remove(&node_id);
            }
        });

        Ok(())
    }

    pub fn model(&self) -> NodeModel {
        let home_relay = self
            .router
            .endpoint()
            .home_relay()
            .get()
            .ok()
            .flatten()
            .map(|url| url.to_string())
            .unwrap_or_else(|| "none".to_string());

        let metrics = self.router.endpoint().metrics();

        let servers = {
            let servers = self.servers.lock().unwrap();
            let mut servers = servers
                .iter()
                .map(|(node_id, server_handle)| {
                    let accepted = server_handle.accepted.load(Ordering::Relaxed);

                    let remote_info = self.router.endpoint().remote_info(*node_id);
                    let connection_type = remote_info
                        .as_ref()
                        .map(|info| info.conn_type.to_string())
                        .unwrap_or_else(|| "unknown".to_string());
                    let latency_ms = remote_info
                        .and_then(|info| info.latency)
                        .map(|latency| latency.as_millis() as u64);

                    let transfer_jobs = {
                        server_handle
                            .jobs
                            .iter()
                            .map(|entry| {
                                let job = entry.value();

                                let (progress, file_size) = match &job.progress {
                                    ServerTransferJobProgress::Transcoding { .. } => {
                                        (TransferJobProgressModel::Transcoding, None)
                                    }

                                    ServerTransferJobProgress::Ready { file_size, .. } => {
                                        (TransferJobProgressModel::Ready, Some(*file_size))
                                    }

                                    ServerTransferJobProgress::InProgress {
                                        started_at,
                                        file_size,
                                        sent,
                                    } => (
                                        TransferJobProgressModel::InProgress {
                                            started_at: *started_at,
                                            bytes: Arc::new(ProgressCounterModel(sent.clone())),
                                        },
                                        Some(*file_size),
                                    ),

                                    ServerTransferJobProgress::Finished {
                                        finished_at,
                                        file_size,
                                    } => (
                                        TransferJobProgressModel::Finished {
                                            finished_at: *finished_at,
                                        },
                                        Some(*file_size),
                                    ),

                                    ServerTransferJobProgress::Failed { error } => (
                                        TransferJobProgressModel::Failed {
                                            error: format!("{error:#}"),
                                        },
                                        None,
                                    ),
                                };

                                TransferJobModel {
                                    job_id: *entry.key(),
                                    file_root: job.file_root.clone(),
                                    file_path: job.file_path.clone(),
                                    file_size,
                                    progress,
                                }
                            })
                            .collect()
                    };

                    ServerModel {
                        name: "unknown".to_string(), // TODO: get real name
                        node_id: node_id.to_string(),
                        connected_at: server_handle.connected_at,

                        accepted,

                        connection_type,
                        latency_ms,

                        transfer_jobs,
                    }
                })
                .collect::<Vec<_>>();
            servers.sort_by_key(|c| c.connected_at);
            servers
        };

        let clients = {
            let clients = self.clients.lock().unwrap();
            let mut clients = clients
                .iter()
                .map(|(node_id, client_handle)| {
                    let accepted = client_handle.accepted.load(Ordering::Relaxed);

                    let remote_info = self.router.endpoint().remote_info(*node_id);
                    let connection_type = remote_info
                        .as_ref()
                        .map(|info| info.conn_type.to_string())
                        .unwrap_or_else(|| "unknown".to_string());
                    let latency_ms = remote_info
                        .and_then(|info| info.latency)
                        .map(|latency| latency.as_millis() as u64);

                    let index = if accepted {
                        let index = client_handle.index.lock().unwrap();
                        index.as_ref().map(|index| {
                            index
                                .iter()
                                .map(|item| IndexItemModel {
                                    node_id: node_id.to_string(),
                                    hash_kind: item.hash_kind.clone(),
                                    hash: item.hash.clone(),
                                    root: item.root.clone(),
                                    path: item.path.clone(),
                                })
                                .collect()
                        })
                    } else {
                        None
                    };

                    let transfer_jobs = {
                        client_handle
                            .jobs
                            .iter()
                            .map(|entry| {
                                let job = entry.value();

                                let (progress, file_size) = match &job.progress {
                                    ClientTransferJobProgress::Requested => {
                                        (TransferJobProgressModel::Requested, None)
                                    }

                                    ClientTransferJobProgress::Transcoding => {
                                        (TransferJobProgressModel::Transcoding, None)
                                    }

                                    ClientTransferJobProgress::Ready { file_size } => {
                                        (TransferJobProgressModel::Ready, Some(*file_size))
                                    }

                                    ClientTransferJobProgress::InProgress {
                                        started_at,
                                        file_size,
                                        written,
                                    } => (
                                        TransferJobProgressModel::InProgress {
                                            started_at: *started_at,
                                            bytes: Arc::new(ProgressCounterModel(written.clone())),
                                        },
                                        Some(*file_size),
                                    ),

                                    ClientTransferJobProgress::Finished {
                                        finished_at,
                                        file_size,
                                    } => (
                                        TransferJobProgressModel::Finished {
                                            finished_at: *finished_at,
                                        },
                                        Some(*file_size),
                                    ),

                                    ClientTransferJobProgress::Failed { error } => (
                                        TransferJobProgressModel::Failed {
                                            error: error.clone(),
                                        },
                                        None,
                                    ),
                                };

                                TransferJobModel {
                                    job_id: *entry.key(),
                                    file_root: job.file_root.clone(),
                                    file_path: job.file_path.clone(),
                                    file_size,
                                    progress,
                                }
                            })
                            .collect()
                    };

                    ClientModel {
                        name: "unknown".to_string(), // TODO: get real name
                        node_id: node_id.to_string(),
                        connected_at: client_handle.connected_at,

                        accepted,

                        connection_type,
                        latency_ms,

                        index,
                        transfer_jobs,
                    }
                })
                .collect::<Vec<_>>();
            clients.sort_by_key(|c| c.connected_at);
            clients
        };

        NodeModel {
            node_id: self.router.endpoint().node_id().to_string(),
            home_relay,

            send_ipv4: metrics.magicsock.send_ipv4.get(),
            send_ipv6: metrics.magicsock.send_ipv6.get(),
            send_relay: metrics.magicsock.send_relay.get(),
            recv_ipv4: metrics.magicsock.recv_data_ipv4.get(),
            recv_ipv6: metrics.magicsock.recv_data_ipv6.get(),
            recv_relay: metrics.magicsock.recv_data_relay.get(),
            conn_success: metrics.magicsock.connection_handshake_success.get(),
            conn_direct: metrics.magicsock.connection_became_direct.get(),

            servers,
            clients,
        }
    }
}

#[derive(Debug, Clone)]
struct Protocol {
    db: Arc<Mutex<Database>>,
    transcode_status_cache: TranscodeStatusCache,
    server_handle_tx: mpsc::UnboundedSender<(NodeId, ServerHandle)>,
    server_closed_tx: mpsc::UnboundedSender<NodeId>,
}

impl Protocol {
    const ALPN: &'static [u8] = b"musicopy/0";

    fn new(
        db: Arc<Mutex<Database>>,
        transcode_status_cache: TranscodeStatusCache,
        server_handle_tx: mpsc::UnboundedSender<(NodeId, ServerHandle)>,
        server_closed_tx: mpsc::UnboundedSender<NodeId>,
    ) -> Self {
        Self {
            db,
            transcode_status_cache,
            server_handle_tx,
            server_closed_tx,
        }
    }
}

impl ProtocolHandler for Protocol {
    fn accept(&self, connection: iroh::endpoint::Connection) -> Boxed<anyhow::Result<()>> {
        let db = self.db.clone();
        let transcode_status_cache = self.transcode_status_cache.clone();
        let server_handle_tx = self.server_handle_tx.clone();
        let server_closed_tx = self.server_closed_tx.clone();
        Box::pin(async move {
            let node_id = connection.remote_node_id()?;
            log::info!("accepted connection from {node_id}");

            let server = Server::new(db, transcode_status_cache, connection, server_handle_tx);
            server.run().await?;

            // remove handle from hashmap
            server_closed_tx
                .send(node_id)
                .expect("failed to send server closed notification");

            Ok(())
        })
    }
}

/// An item requested for downloading by the client.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DownloadItem {
    job_id: u64,

    node_id: NodeId,
    root: String,
    path: String,
}

/// A message sent by the client end of a connection on the control stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
enum ClientMessage {
    /// Identify the client with a friendly name.
    Identify(String),
    /// Request to download files.
    Download(Vec<DownloadItem>),
}

/// An item available for downloading from the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct IndexItem {
    node_id: NodeId,
    root: String,
    path: String,

    hash_kind: String,
    hash: Vec<u8>,
}

/// A job that changed status.
#[derive(Debug, Clone, Serialize, Deserialize)]
enum JobStatusItem {
    Transcoding,
    Ready { file_size: u64 },
    Failed { error: String },
}

/// A message sent by the server end of a connection on the control stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
enum ServerMessage {
    /// Identify the server with a friendly name.
    Identify(String),
    /// Notify the client that the connection has been accepted.
    Accepted,
    /// Inform the client of available files.
    Index(Vec<IndexItem>),
    /// Notify the client that the statuses of jobs have changed.
    JobStatus(HashMap<u64, JobStatusItem>),
}

/// A message sent by the client at the start of a file transfer stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TransferRequest {
    job_id: u64,
}

/// A message sent by the server in a file transfer stream in response to a TransfrRequest.
#[derive(Debug, Clone, Serialize, Deserialize)]
enum TransferResponse {
    /// The job is ready to be downloaded and will be sent by the server.
    Ok { file_size: u64 },
    /// The job was unable to be downloaded.
    Error { error: String },
}

#[derive(Debug)]
struct ServerTransferJob {
    progress: ServerTransferJobProgress,

    // for UI
    file_node_id: NodeId,
    file_root: String,
    file_path: String,
}

#[derive(Debug)]
enum ServerTransferJobProgress {
    /// The server is waiting for the file to be transcoded.
    Transcoding { hash_kind: String, hash: Vec<u8> },
    /// The server is ready to send the file.
    Ready { local_path: PathBuf, file_size: u64 },
    /// The server has started sending the file.
    InProgress {
        started_at: u64,
        file_size: u64,
        sent: Arc<AtomicU64>,
    },
    /// The server has finished sending the file.
    Finished { finished_at: u64, file_size: u64 },
    /// The server failed to send the file.
    Failed { error: anyhow::Error },
}

#[derive(Debug)]
enum ServerCommand {
    Accept,

    Close,

    /// Send a message to the client.
    ///
    /// This is sort of a hack, but it's used by the task that watches for
    /// finished transcodes to send JobStatus messages to the client.
    ServerMessage(ServerMessage),
}

#[derive(Debug, Clone)]
struct ServerHandle {
    tx: mpsc::UnboundedSender<ServerCommand>,

    connected_at: u64,

    accepted: Arc<AtomicBool>,
    jobs: Arc<DashMap<u64, ServerTransferJob>>,
}

struct Server {
    db: Arc<Mutex<Database>>,
    transcode_status_cache: TranscodeStatusCache,
    connection: Connection,
    handle_tx: mpsc::UnboundedSender<(NodeId, ServerHandle)>,

    connected_at: u64,

    accepted: Arc<AtomicBool>,
    jobs: Arc<DashMap<u64, ServerTransferJob>>,
}

impl Server {
    fn new(
        db: Arc<Mutex<Database>>,
        transcode_status_cache: TranscodeStatusCache,
        connection: Connection,
        handle_tx: mpsc::UnboundedSender<(NodeId, ServerHandle)>,
    ) -> Self {
        Self {
            db,
            transcode_status_cache,
            connection,
            handle_tx,

            connected_at: unix_epoch_now_secs(),

            accepted: Arc::new(AtomicBool::new(false)),
            jobs: Arc::new(DashMap::new()),
        }
    }

    async fn run(self) -> anyhow::Result<()> {
        let remote_node_id = self.connection.remote_node_id()?;

        let (tx, mut rx) = mpsc::unbounded_channel();

        // accept bidirectional control stream
        let (send, recv) = self.connection.accept_bi().await?;

        // wrap in framed codecs
        let mut send = FramedWrite::new(send, LengthDelimitedCodec::new()).with_flat_map(
            |message: ServerMessage| {
                let buf: Vec<u8> =
                    postcard::to_stdvec(&message).expect("failed to serialize message");
                futures::stream::once(futures::future::ready(Ok(Bytes::from(buf))))
            },
        );
        let mut recv = FramedRead::new(recv, LengthDelimitedCodec::new())
            .map_err(|e| anyhow::anyhow!("failed to read from connection: {e:?}"))
            .map(|res| {
                res.and_then(|bytes| {
                    postcard::from_bytes::<ClientMessage>(&bytes)
                        .map_err(|e| anyhow::anyhow!("failed to deserialize message: {e:?}"))
                })
            });

        // wait for client Identify
        let Some(Ok(message)) = recv.next().await else {
            log::error!("failed to receive Identify message");
            return Ok(());
        };
        match message {
            ClientMessage::Identify(name) => {
                // TODO: store
                log::debug!("client identified as {name}");
            }
            _ => {
                log::error!("unexpected message, expected Identify: {message:?}");
                return Ok(());
            }
        }

        // send server Identify
        // TODO: real name
        send.send(ServerMessage::Identify("server".to_string()))
            .await
            .expect("failed to send Identify message");

        // handshake finished, send handle to Node
        self.handle_tx
            .send((
                remote_node_id,
                ServerHandle {
                    tx: tx.clone(),

                    connected_at: self.connected_at,

                    accepted: self.accepted.clone(),
                    jobs: self.jobs.clone(),
                },
            ))
            .expect("failed to send server handle");

        // waiting loop, wait for user to accept or deny the connection
        loop {
            tokio::select! {
                Some(command) = rx.recv() => {
                    match command {
                        ServerCommand::Accept => {
                            // continue to next state
                            break;
                        },
                        ServerCommand::Close => {
                            self.connection.close(0u32.into(), b"close");
                            return Ok(());
                        },
                        ServerCommand::ServerMessage(message) => {
                            send.send(message)
                                .await
                                .expect("failed to send ServerMessage");
                        }
                    }
                }

                next_message = recv.next() => {
                    match next_message {
                        Some(Ok(message)) => {
                            log::debug!("unexpected message (not accepted): {message:?}");
                        },
                        Some(Err(e)) => {
                            log::error!("error receiving message: {e}");
                        },
                        None => {
                            log::info!("control stream closed, shutting down server");
                            return Ok(());
                        },
                    }
                }

                else => {
                    anyhow::bail!("stream and receiver closed while waiting for Accept");
                }
            }
        }

        // mark as accepted
        self.accepted.store(true, Ordering::Relaxed);

        // send Accepted message
        send.send(ServerMessage::Accepted)
            .await
            .expect("failed to send Accepted message");

        // send Index message
        let index = self.get_index()?;
        send.send(ServerMessage::Index(index))
            .await
            .expect("failed to send Index message");

        // spawn task to watch for finished transcodes
        // TODO: shutdown signal
        tokio::spawn({
            let jobs = self.jobs.clone();
            let transcode_status_cache = self.transcode_status_cache.clone();
            async move {
                loop {
                    let mut ready_jobs = Vec::new();

                    // check for jobs with Transcoding status
                    for job in jobs.iter() {
                        if let ServerTransferJobProgress::Transcoding { hash_kind, hash } =
                            &job.value().progress
                        {
                            // get transcode status
                            let Some(status) = transcode_status_cache.get(hash_kind, hash) else {
                                continue;
                            };

                            // if transcode status is Ready, set job status to Ready
                            if let TranscodeStatus::Ready {
                                local_path,
                                file_size,
                            } = &*status
                            {
                                ready_jobs.push((*job.key(), local_path.clone(), *file_size));
                            }
                        }
                    }

                    if !ready_jobs.is_empty() {
                        // update job statuses and create status change items
                        let status_changes = ready_jobs
                            .into_iter()
                            .map(|(job_id, local_path, file_size)| {
                                // set job status to Ready
                                // needs to happen outside the loop, since jobs.iter() already holds the entry's lock
                                jobs.alter(&job_id, |_, mut job| {
                                    job.progress = ServerTransferJobProgress::Ready {
                                        local_path,
                                        file_size,
                                    };
                                    job
                                });

                                (job_id, JobStatusItem::Ready { file_size })
                            })
                            .collect::<HashMap<_, _>>();

                        // send status changes to client via ServerCommand::ServerMessage
                        if let Err(e) = tx.send(ServerCommand::ServerMessage(
                            ServerMessage::JobStatus(status_changes),
                        )) {
                            log::warn!("transcode watcher failed to send JobStatus message: {e}");
                        }
                    }

                    // sleep before checking again
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        });

        // main loop
        loop {
            tokio::select! {
                Some(command) = rx.recv() => {
                    match command {
                        ServerCommand::Accept => {
                            log::warn!("unexpected Accept command in main loop");
                        },
                        ServerCommand::Close => {
                            self.connection.close(0u32.into(), b"close");
                            break;
                        },
                        ServerCommand::ServerMessage(message) => {
                            send.send(message)
                                .await
                                .expect("failed to send ServerMessage");
                        }
                    }
                }

                next_message = recv.next() => {
                    match next_message {
                        Some(Ok(message)) => {
                            match message {
                                ClientMessage::Identify(_) => {
                                    log::warn!("unexpected ClientMessage::Identify in main loop");
                                }

                                ClientMessage::Download(items) => {
                                    // get file hashes
                                    // TODO: this could be better
                                    let files = {
                                        let db = self.db.lock().expect("failed to lock database");
                                        db.get_files_by_node_root_path(
                                            items.iter().map(|item| (item.node_id, item.root.clone(), item.path.clone()))
                                        )?.into_iter().map(|f| ((f.node_id, f.root.clone(), f.path.clone()), f)).collect::<HashMap<_, _>>()
                                    };

                                    let status_changes = items.into_iter().map(|item| {
                                        // TODO: wasteful clones
                                        let file = files.get(&(item.node_id, item.root.clone(), item.path.clone()));

                                        let Some(file) = file else {
                                            self.jobs.insert(item.job_id, ServerTransferJob {
                                                progress: ServerTransferJobProgress::Failed { error: anyhow::anyhow!("file not found") },
                                                file_node_id: item.node_id,
                                                file_root: item.root,
                                                file_path: item.path,
                                            });

                                            return (item.job_id, JobStatusItem::Failed {
                                                error: "file not found".to_string(),
                                            });
                                        };

                                        // get transcode status
                                        let transcode_status = self.transcode_status_cache.get(&file.hash_kind, &file.hash);

                                        let Some(transcode_status) = transcode_status else {
                                            // file exists, but transcode status is missing
                                            log::warn!("file {}/{}/{} exists but transcode status is missing, defaulting to Queued",
                                                item.node_id, item.root, item.path);

                                            // create job
                                            self.jobs.insert(item.job_id, ServerTransferJob {
                                                progress: ServerTransferJobProgress::Transcoding { hash_kind: file.hash_kind.clone(), hash: file.hash.clone() },
                                                file_node_id: item.node_id,
                                                file_root: item.root,
                                                file_path: item.path,
                                            });

                                            return (item.job_id, JobStatusItem::Transcoding);
                                        };

                                        match &*transcode_status {
                                            TranscodeStatus::Queued => {
                                                // file is queued for transcoding

                                                // create job
                                                self.jobs.insert(item.job_id, ServerTransferJob {
                                                    progress: ServerTransferJobProgress::Transcoding { hash_kind: file.hash_kind.clone(), hash: file.hash.clone() },
                                                    file_node_id: item.node_id,
                                                    file_root: item.root,
                                                    file_path: item.path,
                                                });

                                                (item.job_id, JobStatusItem::Transcoding)
                                            }
                                            TranscodeStatus::Ready { local_path, file_size } => {
                                                // file is already transcoded

                                                // create job
                                                self.jobs.insert(item.job_id, ServerTransferJob {
                                                    progress: ServerTransferJobProgress::Ready {
                                                        local_path: local_path.clone(),
                                                        file_size: *file_size,
                                                    },
                                                    file_node_id: item.node_id,
                                                    file_root: item.root,
                                                    file_path: item.path,
                                                });

                                                (item.job_id, JobStatusItem::Ready {
                                                    file_size: *file_size,
                                                })
                                            }
                                        }
                                    }).collect::<HashMap<_, _>>();

                                    send.send(ServerMessage::JobStatus(status_changes))
                                        .await
                                        .expect("failed to send JobStatus message");
                                }
                            }
                        },
                        Some(Err(e)) => {
                            log::error!("error receiving message: {e}");
                        },
                        None => {
                            log::info!("control stream closed, shutting down server");
                            break;
                        },
                    }
                }

                // handle file transfer streams
                accept_result = self.connection.accept_bi() => {
                    match accept_result {
                        Ok((mut send, mut recv)) => {
                            let jobs = self.jobs.clone();
                            tokio::spawn(async move {
                                // receive transfer request with job id
                                let transfer_req_len = recv.read_u32().await?;
                                let mut transfer_req_buf = vec![0; transfer_req_len as usize];
                                recv
                                    .read_exact(&mut transfer_req_buf)
                                    .await
                                    .context("failed to read transfer request")?;
                                let transfer_req: TransferRequest =
                                    postcard::from_bytes(&transfer_req_buf).context("failed to deserialize transfer request")?;

                                log::info!("received transfer request for job id {}", transfer_req.job_id);

                                // check job status
                                let (transfer_res, ready) = {
                                    let Some(job) = jobs.get(&transfer_req.job_id) else {
                                        anyhow::bail!("transfer request job id not found: {}", transfer_req.job_id);
                                    };

                                    match &job.progress {
                                        ServerTransferJobProgress::Ready { local_path, file_size } => {
                                            (TransferResponse::Ok { file_size: *file_size }, Some((local_path.clone(), *file_size)))
                                        }
                                        _ => {
                                            (TransferResponse::Error { error: "job not ready".to_string() }, None)
                                        }
                                    }
                                };

                                // send transfer response
                                let transfer_res_buf = postcard::to_stdvec(&transfer_res)
                                    .context("failed to serialize transfer response")?;
                                send.write_u32(transfer_res_buf.len() as u32)
                                    .await
                                    .context("failed to write transfer response length")?;
                                send.write_all(&transfer_res_buf)
                                    .await
                                    .context("failed to write transfer response")?;

                                // TODO: could maybe be nicer
                                let Some((local_path, file_size)) = ready else {
                                    return Ok(());
                                };

                                // check local file exists
                                if !local_path.exists() {
                                    // TODO: set job to failed and respond with error
                                    anyhow::bail!("file at local_path does not exist: {}", local_path.display());
                                }

                                let sent_counter = Arc::new(AtomicU64::new(0));

                                // set job status to InProgress
                                jobs.alter(&transfer_req.job_id, |_, mut job| {
                                    job.progress = ServerTransferJobProgress::InProgress {
                                        started_at: unix_epoch_now_secs(),
                                        file_size,
                                        sent: sent_counter.clone()
                                    };
                                    job
                                });

                                // read file to buffer
                                // TODO: stream instead of reading into memory?
                                let file_content = tokio::fs::read(local_path).await?;

                                // TODO: handle errors during send
                                let mut send_progress = WriteProgress::new(sent_counter.clone(), send);
                                send_progress.write_all(&file_content).await?;

                                // set job status to Finished
                                jobs.alter(&transfer_req.job_id, |_, mut job| {
                                    job.progress = ServerTransferJobProgress::Finished { finished_at: unix_epoch_now_secs(), file_size };
                                    job
                                });

                                Ok::<(), anyhow::Error>(())
                            });
                        }

                        Err(e) => {
                            log::error!("accept_bi error: {e}");
                        }
                    }
                }

                else => {
                    log::warn!("all senders dropped in Server::run, shutting down");
                    break;
                }
            }
        }

        self.connection.closed().await;

        Ok(())
    }

    fn get_index(&self) -> anyhow::Result<Vec<IndexItem>> {
        let db = self.db.lock().unwrap();
        let files = db.get_files()?;
        Ok(files
            .into_iter()
            .map(|file| IndexItem {
                node_id: file.node_id,
                root: file.root,
                path: file.path,

                hash_kind: file.hash_kind,
                hash: file.hash,
            })
            .collect())
    }
}

#[derive(Debug)]
struct ClientTransferJob {
    progress: ClientTransferJobProgress,

    file_node_id: NodeId,
    file_root: String,
    file_path: String,
}

#[derive(Debug)]
enum ClientTransferJobProgress {
    /// The client sent the request and is waiting for its status.
    Requested,
    /// The client is waiting for the file to be transcoded.
    Transcoding,
    /// The client is ready to download the file.
    Ready { file_size: u64 },
    /// The client has started downloading the file.
    InProgress {
        started_at: u64,
        file_size: u64,
        written: Arc<AtomicU64>,
    },
    /// The client has finished downloading the file.
    Finished { finished_at: u64, file_size: u64 },
    /// The client failed to download the file.
    Failed { error: String },
}

#[derive(Debug)]
enum ClientCommand {
    Close,

    DownloadAll,
}

#[derive(Debug, Clone)]
struct ClientHandle {
    tx: mpsc::UnboundedSender<ClientCommand>,

    connected_at: u64,

    accepted: Arc<AtomicBool>,
    index: Arc<Mutex<Option<Vec<IndexItem>>>>,
    jobs: Arc<DashMap<u64, ClientTransferJob>>,
}

struct Client {
    handle_tx: mpsc::UnboundedSender<(NodeId, ClientHandle)>,
    connection: Connection,

    connected_at: u64,

    next_job_id: Arc<AtomicU64>,
    ready_tx: mpsc::UnboundedSender<u64>,

    accepted: Arc<AtomicBool>,
    index: Arc<Mutex<Option<Vec<IndexItem>>>>,
    jobs: Arc<DashMap<u64, ClientTransferJob>>,
}

impl Client {
    fn new(
        handle_tx: mpsc::UnboundedSender<(NodeId, ClientHandle)>,
        connection: Connection,
        download_directory: Arc<Mutex<Option<String>>>,
    ) -> Self {
        let jobs = Arc::new(DashMap::<u64, ClientTransferJob>::new());

        // spawn a task to handle ready jobs and spawn more tasks to download them
        // Client::run() receives ServerMessage::JobStatus messages. jobs marked Ready are sent to this channel
        let (ready_tx, mut ready_rx) = mpsc::unbounded_channel::<u64>();
        tokio::spawn({
            let jobs = jobs.clone();
            let connection = connection.clone();
            async move {
                // convert channel receiver of ready job IDs into a stream for use with buffer_unordered
                let ready_stream = async_stream::stream! {
                    while let Some(job_id) = ready_rx.recv().await {
                        yield job_id;
                    }
                };

                // map stream of ready ids to futures that download the files
                let buffer = ready_stream
                    .map(|job_id| {
                        // get download directory
                        let download_directory = {
                            let download_directory = download_directory.lock().unwrap();
                            download_directory.clone()
                        };

                        let jobs = jobs.clone();
                        let connection = connection.clone();
                        async move {
                            // check if download directory is set
                            // we need to do this inside the async block so that the return type of the closure is always the async block's anonymous future
                            let Some(download_directory) = download_directory else {
                                anyhow::bail!("download directory is None, cannot download");
                            };

                            // check job exists and get details
                            let (file_node_id, file_root, file_path) = {
                                let Some(job) = jobs.get(&job_id) else {
                                    anyhow::bail!("received ready for unknown job ID {job_id}");
                                };

                                (
                                    job.file_node_id,
                                    job.file_root.clone(),
                                    job.file_path.clone(),
                                )
                            };

                            log::debug!("downloading file: {}/{}", file_root, file_path);

                            // open a bidirectional stream
                            let (mut send, mut recv) = connection.open_bi().await?;

                            // send transfer request with job id
                            let transfer_req = TransferRequest { job_id };
                            let transfer_req_buf = postcard::to_stdvec(&transfer_req)
                                .context("failed to serialize transfer request")?;
                            send.write_u32(transfer_req_buf.len() as u32)
                                .await
                                .context("failed to write transfer request length")?;
                            send.write_all(&transfer_req_buf)
                                .await
                                .context("failed to write transfer request")?;

                            // receive transfer response with metadata
                            let transfer_res_len = recv.read_u32().await?;
                            let mut transfer_res_buf = vec![0; transfer_res_len as usize];
                            recv.read_exact(&mut transfer_res_buf)
                                .await
                                .context("failed to read transfer response")?;
                            let transfer_res: TransferResponse =
                                postcard::from_bytes(&transfer_res_buf)
                                    .context("failed to deserialize transfer response")?;

                            // check transfer response
                            let file_size = match transfer_res {
                                TransferResponse::Ok { file_size } => file_size,
                                TransferResponse::Error { error } => {
                                    // set job status to Failed
                                    jobs.alter(&job_id, |_, mut job| {
                                        job.progress = ClientTransferJobProgress::Failed { error };
                                        job
                                    });

                                    return Ok(());
                                }
                            };

                            // set job status to InProgress
                            let written = Arc::new(AtomicU64::new(0));
                            jobs.alter(&job_id, |_, mut job| {
                                job.progress = ClientTransferJobProgress::InProgress {
                                    started_at: unix_epoch_now_secs(),
                                    file_size,
                                    written: written.clone(),
                                };

                                job
                            });

                            // build file path
                            let local_path = {
                                let root_dir_name =
                                    format!("musicopy-{}-{}", &file_node_id, &file_root);
                                let mut local_path =
                                    TreePath::new(download_directory, root_dir_name.into());
                                local_path.push(&file_path);
                                local_path
                            };

                            // create parent directories
                            let parent_dir_path = local_path.parent();
                            if let Some(parent) = parent_dir_path {
                                crate::fs::create_dir_all(&parent)
                                    .await
                                    .context("failed to create directory for root")?;
                            }

                            // open file for writing
                            let file = TreeFile::open_or_create(&local_path, OpenMode::Write)
                                .await
                                .context("failed to open file")?;

                            // copy from stream to file
                            let mut file_progress = WriteProgress::new(written.clone(), file);
                            tokio::io::copy(&mut recv.take(file_size), &mut file_progress).await?;

                            // TODO: handle errors above and update job status

                            // set job status to Finished
                            jobs.alter(&job_id, |_, mut job| {
                                job.progress = ClientTransferJobProgress::Finished {
                                    finished_at: unix_epoch_now_secs(),
                                    file_size,
                                };
                                job
                            });

                            log::debug!("saved file to {local_path:?}");

                            Ok::<(), anyhow::Error>(())
                        }
                    })
                    .buffer_unordered(4);

                tokio::pin!(buffer);

                // poll the stream to download items with limited concurrency
                while let Some(res) = buffer.next().await {
                    if let Err(e) = res {
                        log::error!("error downloading item: {e}");
                    }
                }
            }
        });

        Self {
            handle_tx,
            connection,

            connected_at: unix_epoch_now_secs(),

            next_job_id: Arc::new(AtomicU64::new(0)),
            ready_tx,

            accepted: Arc::new(AtomicBool::new(false)),
            index: Arc::new(Mutex::new(None)),
            jobs,
        }
    }

    async fn run(self) -> anyhow::Result<()> {
        let remote_node_id = self.connection.remote_node_id()?;

        let (tx, mut rx) = mpsc::unbounded_channel();

        // open a bidirectional QUIC stream
        let (send, recv) = self.connection.open_bi().await?;

        // wrap in framed codecs
        let mut send = FramedWrite::new(send, LengthDelimitedCodec::new()).with_flat_map(
            |message: ClientMessage| {
                let buf: Vec<u8> =
                    postcard::to_stdvec(&message).expect("failed to serialize message");
                futures::stream::once(futures::future::ready(Ok(Bytes::from(buf))))
            },
        );
        let mut recv = FramedRead::new(recv, LengthDelimitedCodec::new())
            .map_err(|e| anyhow::anyhow!("failed to read from connection: {e:?}"))
            .map(|res| {
                res.and_then(|bytes| {
                    postcard::from_bytes::<ServerMessage>(&bytes)
                        .map_err(|e| anyhow::anyhow!("failed to deserialize message: {e:?}"))
                })
            });

        // send client Identify
        // TODO: real name
        send.send(ClientMessage::Identify("client".to_string()))
            .await
            .expect("failed to send Identify message");

        // wait for server Identify
        // TODO: also wait for commands
        let Some(Ok(message)) = recv.next().await else {
            log::error!("failed to receive Identify message");
            return Ok(());
        };
        match message {
            ServerMessage::Identify(name) => {
                // TODO: store
                log::info!("server identified as {name}");
            }
            _ => {
                log::error!("unexpected message, expected Identify: {message:?}");
                return Ok(());
            }
        }

        // handshake finished, send handle to Node
        self.handle_tx
            .send((
                remote_node_id,
                ClientHandle {
                    tx,

                    connected_at: self.connected_at,

                    accepted: self.accepted.clone(),
                    index: self.index.clone(),
                    jobs: self.jobs.clone(),
                },
            ))
            .expect("failed to send server handle");

        // waiting loop, wait for server Accepted
        loop {
            tokio::select! {
                Some(command) = rx.recv() => {
                    match command {
                        ClientCommand::Close => {
                            return Ok(());
                        }

                        ClientCommand::DownloadAll => {
                            log::warn!("unexpected DownloadAll command in waiting loop");
                        }
                    }
                }

                next_message = recv.next() => {
                    match next_message {
                        Some(Ok(message)) => {
                            match message {
                                ServerMessage::Accepted => {
                                    log::info!("server accepted the connection");

                                    // continue to next state
                                    break;
                                }
                                _ => {
                                    log::debug!("unexpected message (waiting for Accepted): {message:?}");
                                }
                            }
                        }
                        Some(Err(e)) => {
                            log::error!("error receiving message: {e}");
                        }
                        None => {
                            anyhow::bail!("control stream closed, shutting down client");
                        }
                    }
                }

                else => {
                    anyhow::bail!("stream and receiver closed while waiting for Accepted");
                }
            }
        }

        // mark as accepted
        self.accepted.store(true, Ordering::Relaxed);

        // main loop
        loop {
            tokio::select! {
                Some(command) = rx.recv() => {
                    match command {
                        ClientCommand::Close => {
                            self.connection.close(0u32.into(), b"close");
                            break;
                        }

                        ClientCommand::DownloadAll => {
                            let index = {
                                let index = self.index.lock().unwrap();
                                index.clone()
                            };
                            let Some(index) = index else {
                                log::error!("DownloadAll: no index available, cannot download");
                                continue;
                            };

                            // create jobs and download request items
                            let download_requests = index.clone().into_iter().map(|file| {
                                let job_id = self.next_job_id.fetch_add(1, Ordering::Relaxed);

                                self.jobs.insert(job_id, ClientTransferJob {
                                    progress: ClientTransferJobProgress::Requested,
                                    file_node_id: file.node_id,
                                    file_root: file.root.clone(),
                                    file_path: file.path.clone(),
                                });

                                DownloadItem {
                                    job_id,

                                    node_id: file.node_id,
                                    root: file.root,
                                    path: file.path,
                                }
                            }).collect::<Vec<_>>();

                            // send download request
                            send.send(ClientMessage::Download(download_requests))
                                .await
                                .expect("failed to send Download message");
                        }
                    }
                }

                next_message = recv.next() => {
                    match next_message {
                        Some(Ok(message)) => {
                            match message {
                                ServerMessage::Index(new_index) => {
                                    log::info!("received index with {} items", new_index.len());
                                    {
                                        let mut index = self.index.lock().unwrap();
                                        *index = Some(new_index);
                                    }
                                }

                                ServerMessage::JobStatus(status_changes) => {
                                    for (job_id, status) in status_changes {
                                        match status {
                                            JobStatusItem::Transcoding => {
                                                // set job status to Transcoding
                                                self.jobs.alter(&job_id, |_, mut job| {
                                                    job.progress = ClientTransferJobProgress::Transcoding;
                                                    job
                                                });
                                            },
                                            JobStatusItem::Ready { file_size } => {
                                                // set job status to Ready
                                                self.jobs.alter(&job_id, |_, mut job| {
                                                    job.progress = ClientTransferJobProgress::Ready { file_size };
                                                    job
                                                });

                                                // send job id to ready channel
                                                self.ready_tx.send(job_id).context("failed to send job id to ready channel")?;
                                            },
                                            JobStatusItem::Failed { error } => {
                                                // set job status to Failed
                                                self.jobs.alter(&job_id, |_, mut job| {
                                                    job.progress = ClientTransferJobProgress::Failed { error };
                                                    job
                                                });
                                            },
                                        }
                                    }
                                }

                                _ => {
                                    log::debug!("unexpected message in main loop: {message:?}");
                                }
                            }
                        }
                        Some(Err(e)) => {
                            log::error!("error receiving message: {e}");
                        }
                        None => {
                            log::info!("control stream closed, shutting down client");
                            break;
                        }
                    }
                }

                _ = self.connection.closed() => {
                    log::info!("connection closed");
                    break;
                }

                else => {
                    log::warn!("all senders dropped in Client::run, shutting down");
                    break;
                }
            }
        }

        self.connection.closed().await;

        Ok(())
    }
}

/// Returns the current system time in seconds since the Unix epoch.
fn unix_epoch_now_secs() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Wrapper for `T: AsyncWrite` that tracks the number of bytes written in an `Arc<AtomicU64>`.
struct WriteProgress<T> {
    inner: T,
    written: Arc<AtomicU64>,
}

impl<T> WriteProgress<T> {
    fn new(written: Arc<AtomicU64>, inner: T) -> Self {
        Self { inner, written }
    }
}

impl<T: AsyncWrite + Unpin> AsyncWrite for WriteProgress<T> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        let res = Pin::new(&mut self.inner).poll_write(cx, buf);
        if let std::task::Poll::Ready(Ok(size)) = &res {
            self.written.fetch_add(*size as u64, Ordering::Relaxed);
        }
        res
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }

    fn poll_write_vectored(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        bufs: &[std::io::IoSlice<'_>],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        let res = Pin::new(&mut self.inner).poll_write_vectored(cx, bufs);
        if let std::task::Poll::Ready(Ok(size)) = &res {
            self.written.fetch_add(*size as u64, Ordering::Relaxed);
        }
        res
    }

    fn is_write_vectored(&self) -> bool {
        self.inner.is_write_vectored()
    }
}
