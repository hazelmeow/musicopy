//! Networking.
//!
//! A `Node` is an Iroh node that can perform the client or server end of the protocol.
//!
//! A `Client` is a struct representing an outgoing connection to a server.
//! Clients request files, primarily used in the mobile app.
//!
//! A `Server` is a struct representing an incoming connection from a client.
//! Servers send files, primarily used in the desktop app.

use crate::database::Database;
use futures::{SinkExt, StreamExt, TryStreamExt};
use iroh::{
    Endpoint, NodeAddr, NodeId, SecretKey,
    endpoint::Connection,
    protocol::{ProtocolHandler, Router},
};
use itertools::Itertools;
use n0_future::future::Boxed;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, atomic::AtomicBool},
    time::{Duration, SystemTime},
};
use tokio::sync::mpsc;
use tokio_util::{
    bytes::Bytes,
    codec::{FramedRead, FramedWrite, LengthDelimitedCodec},
};

/// Model of an incoming connection.
#[derive(Debug, uniffi::Record)]
pub struct ServerModel {
    pub name: String,
    pub node_id: String,
    pub connected_at: u64,
    pub connection_type: String,
    pub latency_ms: Option<u64>,
}

/// Model of an outgoing connection.
#[derive(Debug, uniffi::Record)]
pub struct ClientModel {
    pub name: String,
    pub node_id: String,
    pub connected_at: u64,
    pub connection_type: String,
    pub latency_ms: Option<u64>,
}

/// Node state sent to Compose.
#[derive(Debug, uniffi::Record)]
pub struct NodeModel {
    node_id: String,
    home_relay: String,

    send_ipv4: u64,
    send_ipv6: u64,
    send_relay: u64,
    recv_ipv4: u64,
    recv_ipv6: u64,
    recv_relay: u64,
    conn_success: u64,
    conn_direct: u64,

    active_servers: Vec<ServerModel>,
    pending_servers: Vec<ServerModel>,

    active_clients: Vec<ClientModel>,
    pending_clients: Vec<ClientModel>,
}

#[derive(Debug)]
pub enum NodeCommand {
    Connect(NodeAddr),

    AcceptConnection(NodeId),
    DenyConnection(NodeId),

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
    client_handle_rx: mpsc::UnboundedReceiver<(NodeId, ClientHandle)>,
}

impl Node {
    pub async fn new(
        secret_key: SecretKey,
        db: Arc<Mutex<Database>>,
    ) -> anyhow::Result<(Arc<Self>, NodeRunToken)> {
        let (server_handle_tx, server_handle_rx) = mpsc::unbounded_channel();
        let (client_handle_tx, client_handle_rx) = mpsc::unbounded_channel();

        let endpoint = Endpoint::builder()
            .secret_key(secret_key)
            .discovery_n0()
            .bind()
            .await?;
        let protocol = Protocol::new(db.clone(), server_handle_tx);

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
            mut client_handle_rx,
        } = run_token;

        loop {
            tokio::select! {
                Some(command) = rx.recv() => {
                    match command {
                        NodeCommand::Connect(addr) => {
                            let this = self.clone();
                            tokio::task::spawn(async move {
                                // TODO: return error
                                let _ = this.connect(addr).await;
                            });
                        },

                        NodeCommand::AcceptConnection(node_id) => {
                            let servers = self.servers.lock().unwrap();
                            if let Some(server_handle) = servers.get(&node_id) {
                                server_handle.tx.send(ServerCommand::Accept).expect("failed to send accept command");
                            } else {
                                log::error!("AcceptConnection: no server found with node_id: {node_id}");
                            }
                        },
                        NodeCommand::DenyConnection(node_id) => {
                            let servers = self.servers.lock().unwrap();
                            if let Some(server_handle) = servers.get(&node_id) {
                                server_handle.tx.send(ServerCommand::Close).expect("failed to send close command");
                            } else {
                                log::error!("DenyConnection: no server found with node_id: {node_id}");
                            }
                        },

                        NodeCommand::Stop => break,
                    }
                }

                Some((server_id, server_handle)) = server_handle_rx.recv() => {
                    let mut servers = self.servers.lock().unwrap();
                    servers.insert(server_id, server_handle);
                }

                Some((client_id, client_handle)) = client_handle_rx.recv() => {
                    let mut clients = self.clients.lock().unwrap();
                    clients.insert(client_id, client_handle);
                }
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
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
        tokio::spawn(async move {
            let client = Client::new(connection, client_handle_tx);

            if let Err(e) = client.run().await {
                log::error!("error during client.run(): {e}");
            }

            // TODO: remove handle from hashmap
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

        let (active_servers, pending_servers) = {
            let servers = self.servers.lock().unwrap();
            let (mut active_servers, mut pending_servers) = servers
                .iter()
                .map(|(node_id, server_handle)| {
                    let accepted = server_handle
                        .accepted
                        .load(std::sync::atomic::Ordering::Relaxed);

                    let remote_info = self.router.endpoint().remote_info(*node_id);
                    let connection_type = remote_info
                        .as_ref()
                        .map(|info| info.conn_type.to_string())
                        .unwrap_or_else(|| "unknown".to_string());
                    let latency_ms = remote_info
                        .and_then(|info| info.latency)
                        .map(|latency| latency.as_millis() as u64);

                    let model = ServerModel {
                        name: "unknown".to_string(), // TODO: get real name
                        node_id: node_id.to_string(),
                        connected_at: server_handle.connected_at,
                        connection_type,
                        latency_ms,
                    };

                    if accepted { Ok(model) } else { Err(model) }
                })
                .partition_result::<Vec<_>, Vec<_>, _, _>();
            active_servers.sort_by_key(|c| c.connected_at);
            pending_servers.sort_by_key(|c| c.connected_at);
            (active_servers, pending_servers)
        };

        let (active_clients, pending_clients) = {
            let clients = self.clients.lock().unwrap();
            let (mut active_clients, mut pending_clients) = clients
                .iter()
                .map(|(node_id, client_handle)| {
                    let accepted = client_handle
                        .accepted
                        .load(std::sync::atomic::Ordering::Relaxed);

                    let remote_info = self.router.endpoint().remote_info(*node_id);
                    let connection_type = remote_info
                        .as_ref()
                        .map(|info| info.conn_type.to_string())
                        .unwrap_or_else(|| "unknown".to_string());
                    let latency_ms = remote_info
                        .and_then(|info| info.latency)
                        .map(|latency| latency.as_millis() as u64);

                    let model = ClientModel {
                        name: "unknown".to_string(), // TODO: get real name
                        node_id: node_id.to_string(),
                        connected_at: client_handle.connected_at,
                        connection_type,
                        latency_ms,
                    };

                    if accepted { Ok(model) } else { Err(model) }
                })
                .partition_result::<Vec<_>, Vec<_>, _, _>();
            active_clients.sort_by_key(|c| c.connected_at);
            pending_clients.sort_by_key(|c| c.connected_at);
            (active_clients, pending_clients)
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

            active_servers,
            pending_servers,

            active_clients,
            pending_clients,
        }
    }
}

#[derive(Debug, Clone)]
struct Protocol {
    db: Arc<Mutex<Database>>,
    server_handle_tx: mpsc::UnboundedSender<(NodeId, ServerHandle)>,
}

impl Protocol {
    const ALPN: &'static [u8] = b"musicopy/0";

    fn new(
        db: Arc<Mutex<Database>>,
        server_handle_tx: mpsc::UnboundedSender<(NodeId, ServerHandle)>,
    ) -> Self {
        Self {
            db,
            server_handle_tx,
        }
    }
}

impl ProtocolHandler for Protocol {
    fn accept(&self, connection: iroh::endpoint::Connection) -> Boxed<anyhow::Result<()>> {
        let db = self.db.clone();
        let server_handle_tx = self.server_handle_tx.clone();
        Box::pin(async move {
            let node_id = connection.remote_node_id()?;
            log::info!("accepted connection from {node_id}");

            let server = Server::new(db, connection, server_handle_tx);
            server.run().await?;

            Ok(())
        })
    }
}

/// A message sent by the client end of a connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
enum ClientMessage {
    Identify(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct IndexItem {
    hash_kind: String,
    hash: String,
    root: String,
    path: String,
}

/// A message sent by the server end of a connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
enum ServerMessage {
    Identify(String),
    Accepted,
    Index(Vec<IndexItem>),
}

#[derive(Debug)]
enum ServerCommand {
    Accept,

    Close,
}

#[derive(Debug, Clone)]
struct ServerHandle {
    connected_at: u64,
    accepted: Arc<AtomicBool>,
    tx: mpsc::UnboundedSender<ServerCommand>,
}

struct Server {
    db: Arc<Mutex<Database>>,
    connection: Connection,
    handle_tx: mpsc::UnboundedSender<(NodeId, ServerHandle)>,
    connected_at: u64,
    accepted: Arc<AtomicBool>,
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
            connected_at: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            accepted: Arc::new(AtomicBool::new(false)),
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
                    connected_at: self.connected_at,
                    accepted: self.accepted.clone(),
                    tx: tx.clone(),
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

                Some(Ok(message)) = recv.next() => {
                    log::debug!("unexpected message (waiting  for Accepted): {message:?}");
                }
            }
        }

        // mark as accepted
        self.accepted
            .store(true, std::sync::atomic::Ordering::Relaxed);

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

                Some(Ok(message)) = recv.next() => {
                    log::debug!("accepted message: {message:?}");
                }

                // TODO: accept new bidi streams for file transfers
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
}

#[derive(Debug, Clone)]
struct ClientHandle {
    connected_at: u64,
    accepted: Arc<AtomicBool>,
    tx: mpsc::UnboundedSender<ClientCommand>,
}

struct Client {
    connection: Connection,
    handle_tx: mpsc::UnboundedSender<(NodeId, ClientHandle)>,
    connected_at: u64,
    accepted: Arc<AtomicBool>,
}

impl Client {
    fn new(
        connection: Connection,
        handle_tx: mpsc::UnboundedSender<(NodeId, ClientHandle)>,
    ) -> Self {
        Self {
            connection,
            handle_tx,
            connected_at: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            accepted: Arc::new(AtomicBool::new(false)),
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
                    connected_at: self.connected_at,
                    accepted: self.accepted.clone(),
                    tx,
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
                    }
                }

                Some(Ok(message)) = recv.next() => {
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
            }
        }

        // mark as accepted
        self.accepted
            .store(true, std::sync::atomic::Ordering::Relaxed);

        // main loop
        loop {
            tokio::select! {
                Some(command) = rx.recv() => {
                    match command {
                        ClientCommand::Close => {
                            self.connection.close(0u32.into(), b"close");
                            break;
                        }
                    }
                }

                Some(Ok(message)) = recv.next() => {
                    log::debug!("received message: {message:?}");

                    match message {
                        ServerMessage::Index(index) => {
                            log::info!("received index with {} items", index.len());
                            // TOOD: do something
                        }

                        _ => {
                            log::debug!("unexpected message in main loop: {message:?}");
                        }
                    }
                }

                _ = self.connection.closed() => {
                    log::info!("connection closed");
                    break;
                }
            }
        }

        self.connection.closed().await;

        Ok(())
    }
}
