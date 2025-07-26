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
    time::SystemTime,
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
    InProgress {
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
    pub started_at: u64,
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
    pub hash: String,
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
    Connect {
        addr: NodeAddr,
        callback: oneshot::Sender<anyhow::Result<()>>,
    },

    AcceptConnection(NodeId),
    DenyConnection(NodeId),

    DownloadAll {
        client: NodeId,
        download_directory: String,
    },

    Stop,
}

#[derive(Debug)]
pub struct Node {
    db: Arc<Mutex<Database>>,

    router: Router,
    protocol: Protocol,

    client_handle_tx: mpsc::UnboundedSender<(NodeId, ClientHandle)>,

    servers: Mutex<HashMap<NodeId, ServerHandle>>,
    clients: Mutex<HashMap<NodeId, ClientHandle>>,
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
    ) -> anyhow::Result<(Arc<Self>, NodeRunToken)> {
        let (server_handle_tx, server_handle_rx) = mpsc::unbounded_channel();
        let (server_closed_tx, server_closed_rx) = mpsc::unbounded_channel();
        let (client_handle_tx, client_handle_rx) = mpsc::unbounded_channel();

        let endpoint = Endpoint::builder()
            .secret_key(secret_key)
            .discovery_n0()
            .bind()
            .await?;
        let protocol = Protocol::new(db.clone(), server_handle_tx, server_closed_tx);

        let router = Router::builder(endpoint)
            .accept(Protocol::ALPN, protocol.clone())
            .spawn();

        let node = Arc::new(Self {
            db,

            router,
            protocol,

            client_handle_tx,

            servers: Mutex::new(HashMap::new()),
            clients: Mutex::new(HashMap::new()),
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

                        NodeCommand::DownloadAll { client, download_directory } => {
                            let clients = self.clients.lock().unwrap();
                            if let Some(client_handle) = clients.get(&client) {
                                client_handle.tx.send(ClientCommand::DownloadAll { download_directory }).expect("failed to send ClientCommand::DownloadAll");
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
        tokio::spawn(async move {
            let client = Client::new(client_handle_tx, connection);

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

                                let progress = match &job.progress {
                                    TransferJobProgress::InProgress { written } => {
                                        TransferJobProgressModel::InProgress {
                                            bytes: Arc::new(ProgressCounterModel(written.clone())),
                                        }
                                    }
                                    TransferJobProgress::Finished { finished_at } => {
                                        TransferJobProgressModel::Finished {
                                            finished_at: *finished_at,
                                        }
                                    }
                                    TransferJobProgress::Failed { error } => {
                                        TransferJobProgressModel::Failed {
                                            error: format!("{error:#}"),
                                        }
                                    }
                                };

                                TransferJobModel {
                                    started_at: job.started_at,
                                    file_root: job.file_root.clone(),
                                    file_path: job.file_path.clone(),
                                    file_size: job.file_size,
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

                                let progress = match &job.progress {
                                    TransferJobProgress::InProgress { written } => {
                                        TransferJobProgressModel::InProgress {
                                            bytes: Arc::new(ProgressCounterModel(written.clone())),
                                        }
                                    }
                                    TransferJobProgress::Finished { finished_at } => {
                                        TransferJobProgressModel::Finished {
                                            finished_at: *finished_at,
                                        }
                                    }
                                    TransferJobProgress::Failed { error } => {
                                        TransferJobProgressModel::Failed {
                                            error: format!("{error:#}"),
                                        }
                                    }
                                };

                                TransferJobModel {
                                    started_at: job.started_at,
                                    file_root: job.file_root.clone(),
                                    file_path: job.file_path.clone(),
                                    file_size: job.file_size,
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
    server_handle_tx: mpsc::UnboundedSender<(NodeId, ServerHandle)>,
    server_closed_tx: mpsc::UnboundedSender<NodeId>,
}

impl Protocol {
    const ALPN: &'static [u8] = b"musicopy/0";

    fn new(
        db: Arc<Mutex<Database>>,
        server_handle_tx: mpsc::UnboundedSender<(NodeId, ServerHandle)>,
        server_closed_tx: mpsc::UnboundedSender<NodeId>,
    ) -> Self {
        Self {
            db,
            server_handle_tx,
            server_closed_tx,
        }
    }
}

impl ProtocolHandler for Protocol {
    fn accept(&self, connection: iroh::endpoint::Connection) -> Boxed<anyhow::Result<()>> {
        let db = self.db.clone();
        let server_handle_tx = self.server_handle_tx.clone();
        let server_closed_tx = self.server_closed_tx.clone();
        Box::pin(async move {
            let node_id = connection.remote_node_id()?;
            log::info!("accepted connection from {node_id}");

            let server = Server::new(db, connection, server_handle_tx);
            server.run().await?;

            // remove handle from hashmap
            server_closed_tx
                .send(node_id)
                .expect("failed to send server closed notification");

            Ok(())
        })
    }
}

/// A message sent by the client end of a connection on the control stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
enum ClientMessage {
    Identify(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct IndexItem {
    node_id: NodeId,
    hash_kind: String,
    hash: String,
    root: String,
    path: String,
}

/// A message sent by the server end of a connection on the control stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
enum ServerMessage {
    Identify(String),
    Accepted,
    Index(Vec<IndexItem>),
}

/// A message sent by the client at the start of a file transfer stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DownloadRequest {
    node_id: NodeId,
    root: String,
    path: String,
}

/// A message sent by the server in a file transfer stream with file metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DownloadResponse {
    file_size: u64,
}

#[derive(Debug)]
enum ServerCommand {
    Accept,

    Close,
}

#[derive(Debug)]
struct TransferJob {
    started_at: u64,
    file_root: String,
    file_path: String,
    file_size: Option<u64>,
    progress: TransferJobProgress,
}

#[derive(Debug)]
enum TransferJobProgress {
    InProgress { written: Arc<AtomicU64> },
    Finished { finished_at: u64 },
    Failed { error: anyhow::Error },
}

#[derive(Debug, Clone)]
struct ServerHandle {
    tx: mpsc::UnboundedSender<ServerCommand>,

    connected_at: u64,

    accepted: Arc<AtomicBool>,
    jobs: Arc<DashMap<u64, TransferJob>>,
}

struct Server {
    db: Arc<Mutex<Database>>,
    connection: Connection,
    handle_tx: mpsc::UnboundedSender<(NodeId, ServerHandle)>,

    connected_at: u64,

    next_job_id: AtomicU64,

    accepted: Arc<AtomicBool>,
    jobs: Arc<DashMap<u64, TransferJob>>,
}

impl Server {
    fn new(
        db: Arc<Mutex<Database>>,
        connection: Connection,
        handle_tx: mpsc::UnboundedSender<(NodeId, ServerHandle)>,
    ) -> Self {
        Self {
            db,
            connection,
            handle_tx,

            connected_at: unix_epoch_now_secs(),

            next_job_id: AtomicU64::new(0),

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
        // TODO: real index
        let index = self.get_index()?;
        send.send(ServerMessage::Index(index))
            .await
            .expect("failed to send Index message");

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
                    }
                }

                next_message = recv.next() => {
                    match next_message {
                        Some(Ok(message)) => {
                            log::info!("accepted message: {message:?}");
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
                            let db = self.db.clone();
                            let job_id = self.next_job_id.fetch_add(1, Ordering::Relaxed);
                            let jobs = self.jobs.clone();
                            tokio::spawn(async move {
                                // receive download request
                                let download_req_len = recv.read_u32().await?;
                                let mut download_req_buf = vec![0; download_req_len as usize];
                                recv
                                    .read_exact(&mut download_req_buf)
                                    .await
                                    .context("failed to read download request")?;
                                let download_req: DownloadRequest =
                                    postcard::from_bytes(&download_req_buf).context("failed to deserialize download request")?;

                                log::info!("received download request for {}/{}", download_req.root, download_req.path);

                                // query database for file
                                let file = {
                                    let db = db.lock().unwrap();
                                    db.get_file_by_node_root_path(
                                        download_req.node_id,
                                        &download_req.root,
                                        &download_req.path,
                                    )?.ok_or_else(|| anyhow::anyhow!("file not found in database"))?
                                };

                                // check local file exists
                                let file_path = PathBuf::from(&file.local_path);
                                if !file_path.exists() {
                                    // TODO: create a job with failed progress
                                    anyhow::bail!("file at local_path does not exist: {}", file.local_path);
                                }

                                // get file size
                                let metadata = file_path.metadata().context("failed to get file metadata")?;
                                let file_size = metadata.len();

                                let written = Arc::new(AtomicU64::new(0));

                                // insert job
                                {
                                    jobs.insert(job_id, TransferJob {
                                        started_at: unix_epoch_now_secs(),
                                        file_root: download_req.root.clone(),
                                        file_path: download_req.path.clone(),
                                        file_size: Some(file_size),
                                        progress: TransferJobProgress::InProgress { written: written.clone() },
                                    });
                                }

                                // send download response with metadata
                                let download_response = DownloadResponse {
                                    file_size,
                                };
                                let download_response_buf = postcard::to_stdvec(&download_response)
                                    .context("failed to serialize download response")?;
                                send.write_u32(download_response_buf.len() as u32)
                                    .await
                                    .context("failed to write download response length")?;
                                send.write_all(&download_response_buf)
                                    .await
                                    .context("failed to write download response")?;

                                // read file to buffer
                                // TODO: stream instead of reading into memory?
                                let file_content = tokio::fs::read(file_path).await?;

                                // TODO: handle errors during send
                                let mut send_progress = WriteProgress::new(written.clone(), send);
                                send_progress.write_all(&file_content).await?;

                                // mark job as finished
                                {
                                    jobs.alter(&job_id, |_, mut job| {
                                        job.progress = TransferJobProgress::Finished { finished_at: unix_epoch_now_secs() };
                                        job
                                    });
                                }

                                log::info!("finished sending file content for {}/{}", download_req.root, download_req.path);

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
                hash_kind: file.hash_kind,
                hash: file.hash,
                root: file.root,
                path: file.path,
            })
            .collect())
    }
}

#[derive(Debug)]
enum ClientCommand {
    Close,

    DownloadAll { download_directory: String },
}

#[derive(Debug, Clone)]
struct ClientHandle {
    tx: mpsc::UnboundedSender<ClientCommand>,

    connected_at: u64,

    accepted: Arc<AtomicBool>,
    index: Arc<Mutex<Option<Vec<IndexItem>>>>,
    jobs: Arc<DashMap<u64, TransferJob>>,
}

struct Client {
    handle_tx: mpsc::UnboundedSender<(NodeId, ClientHandle)>,
    connection: Connection,

    connected_at: u64,

    next_job_id: Arc<AtomicU64>,

    accepted: Arc<AtomicBool>,
    index: Arc<Mutex<Option<Vec<IndexItem>>>>,
    jobs: Arc<DashMap<u64, TransferJob>>,
}

impl Client {
    fn new(
        handle_tx: mpsc::UnboundedSender<(NodeId, ClientHandle)>,
        connection: Connection,
    ) -> Self {
        Self {
            handle_tx,
            connection,

            connected_at: unix_epoch_now_secs(),

            next_job_id: Arc::new(AtomicU64::new(0)),

            accepted: Arc::new(AtomicBool::new(false)),
            index: Arc::new(Mutex::new(None)),
            jobs: Arc::new(DashMap::new()),
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

                        ClientCommand::DownloadAll {.. } => {
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

                        ClientCommand::DownloadAll { download_directory } => {
                            log::info!("received DownloadAll command, downloading to {download_directory:?}");

                            let index = {
                                let index = self.index.lock().unwrap();
                                index.clone()
                            };
                            let Some(index) = index else {
                                log::error!("DownloadAll: no index available, cannot download");
                                continue;
                            };

                            // create a job for each item in the index
                            let index_jobs = index.clone().into_iter().map(|item| {
                                let job_id = self.next_job_id.fetch_add(1, Ordering::Relaxed);

                                let written = Arc::new(AtomicU64::new(0));

                                 // insert job
                                {
                                    self.jobs.insert(job_id, TransferJob {
                                        started_at: unix_epoch_now_secs(),
                                        file_root: item.root.clone(),
                                        file_path: item.path.clone(),
                                        file_size: None,
                                        progress: TransferJobProgress::InProgress { written: written.clone() },
                                    });
                                }

                                (job_id, written, item)
                            }).collect::<Vec<_>>();

                            // send download requests and write files to disk
                            // this is only done for a few jobs at a time
                            // TODO: tune concurrency level
                            let mut buffered = futures::stream::iter(index_jobs)
                                .map(|(job_id, written, item)| {
                                    let connection = self.connection.clone();
                                    let jobs = self.jobs.clone();
                                    let download_directory = download_directory.clone();
                                    async move {
                                        log::debug!("downloading file: {}/{}", item.root, item.path);

                                        // open a bidirectional stream
                                        let (mut send, mut recv) = connection.open_bi().await?;

                                        // send download request
                                        let download_request = DownloadRequest {
                                            node_id: item.node_id,
                                            root: item.root.clone(),
                                            path: item.path.clone()
                                        };
                                        let download_request_buf = postcard::to_stdvec(&download_request)
                                            .context("failed to serialize download request")?;
                                        send.write_u32(download_request_buf.len() as u32)
                                            .await
                                            .context("failed to write download request length")?;
                                        send.write_all(&download_request_buf)
                                            .await
                                            .context("failed to write download request")?;

                                        // receive download response with metadata
                                        let download_response_len = recv.read_u32().await?;
                                        let mut download_response_buf = vec![0; download_response_len as usize];
                                        recv
                                            .read_exact(&mut download_response_buf)
                                            .await
                                            .context("failed to read download response")?;
                                        let download_response: DownloadResponse =
                                            postcard::from_bytes(&download_response_buf).context("failed to deserialize download response")?;

                                        // set file size in job
                                        jobs.alter(&job_id, |_, mut job| {
                                            job.file_size = Some(download_response.file_size);
                                            job
                                        });

                                        // build file path
                                        let file_path = {
                                            let root_dir_name = format!("musicopy-{}-{}", &item.node_id, &item.root);
                                            let mut file_path = TreePath::new(download_directory, root_dir_name.into());
                                            file_path.push(&item.path);
                                            file_path
                                        };

                                        // create parent directories
                                        let parent_dir_path = file_path.parent();
                                        if let Some(parent) = parent_dir_path {
                                            crate::fs::create_dir_all(&parent).await
                                                .context("failed to create directory for root")?;
                                        }

                                        // open file for writing
                                        let file = TreeFile::open_or_create(&file_path, OpenMode::Write).await
                                            .context("failed to open file")?;

                                        // copy from stream to file
                                        let mut file_progress = WriteProgress::new(written.clone(), file);
                                        tokio::io::copy(&mut recv.take(download_response.file_size), &mut file_progress).await?;

                                        // mark job as finished
                                        jobs.alter(&job_id, |_, mut job| {
                                            job.progress = TransferJobProgress::Finished { finished_at: unix_epoch_now_secs() };
                                            job
                                        });

                                        log::debug!("saved file to {:?}", file_path);

                                        Ok::<(), anyhow::Error>(())
                                    }
                                })
                                .buffer_unordered(4);

                            while let Some(res) = buffered.next().await {
                                if let Err(e) = res {
                                    log::error!("error downloading item: {e}");
                                    continue;
                                }
                            }
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
