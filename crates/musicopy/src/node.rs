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

#[derive(Debug, uniffi::Record)]
pub struct ConnectionModel {
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

    active_connections: Vec<ConnectionModel>,
    pending_connections: Vec<ConnectionModel>,
}

#[derive(Debug)]
pub enum PeerState {
    Waiting,
    Accepted,
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
    router: Router,
    protocol: Protocol,
    peers: Mutex<HashMap<NodeId, PeerHandle>>,
}

#[derive(Debug)]
pub struct NodeRunToken {
    peer_rx: mpsc::UnboundedReceiver<(NodeId, PeerHandle)>,
}

impl Node {
    pub async fn new(secret_key: SecretKey) -> anyhow::Result<(Arc<Self>, NodeRunToken)> {
        let (peer_tx, peer_rx) = mpsc::unbounded_channel();

        let endpoint = Endpoint::builder()
            .secret_key(secret_key)
            .discovery_n0()
            .bind()
            .await?;
        let protocol = Protocol::new(peer_tx);

        let router = Router::builder(endpoint)
            .accept(Protocol::ALPN, protocol.clone())
            .spawn();

        let node = Arc::new(Self {
            router,
            protocol,
            peers: Mutex::new(HashMap::new()),
        });

        let node_run = NodeRunToken { peer_rx };

        Ok((node, node_run))
    }

    pub async fn run(
        self: &Arc<Self>,
        mut rx: mpsc::UnboundedReceiver<NodeCommand>,
        run_token: NodeRunToken,
    ) -> anyhow::Result<()> {
        let NodeRunToken { mut peer_rx } = run_token;

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
                            let peers = self.peers.lock().unwrap();
                            if let Some(peer_handle) = peers.get(&node_id) {
                                peer_handle.tx.send(PeerCommand::Accept).expect("failed to send accept command");
                            } else {
                                log::error!("AcceptConnection: no peer found with node_id: {node_id}");
                            }
                        },
                        NodeCommand::DenyConnection(node_id) => {
                            let peers = self.peers.lock().unwrap();
                            if let Some(peer_handle) = peers.get(&node_id) {
                                peer_handle.tx.send(PeerCommand::Close).expect("failed to send close command");
                            } else {
                                log::error!("DenyConnection: no peer found with node_id: {node_id}");
                            }
                        },

                        NodeCommand::Stop => break,
                    }
                }

                Some((peer_id, peer_handle)) = peer_rx.recv() => {
                    let mut peers = self.peers.lock().unwrap();
                    peers.insert(peer_id, peer_handle);
                }
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        let _ = self.router.shutdown().await;

        Ok(())
    }

    async fn connect(self: &Arc<Self>, addr: NodeAddr) -> anyhow::Result<()> {
        // connect
        let conn = self.router.endpoint().connect(addr, Protocol::ALPN).await?;

        // open a bidirectional QUIC stream
        let (send, recv) = conn.open_bi().await?;

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

        // wait for server Accepted
        let Some(Ok(message)) = recv.next().await else {
            log::error!("failed to receive Accepted message");
            return Ok(());
        };
        match message {
            ServerMessage::Accepted => {
                log::info!("server accepted the connection");
            }
            _ => {
                log::error!("unexpected message, expected Accepted: {message:?}");
                return Ok(());
            }
        }

        // TODO: open streams and such

        loop {
            tokio::select! {
                Some(Ok(message)) = recv.next() => {
                    log::debug!("received message: {message:?}");
                }

                _ = conn.closed() => {
                    log::info!("connection closed");
                    break;
                }
            }
        }

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

        let (active_connections, pending_connections) = {
            let peers = self.peers.lock().unwrap();
            let (mut active_connections, mut pending_connections) = peers
                .iter()
                .map(|(node_id, peer_handle)| {
                    let accepted = peer_handle
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

                    let model = ConnectionModel {
                        name: "unknown".to_string(), // TODO: get real name
                        node_id: node_id.to_string(),
                        connected_at: peer_handle.connected_at,
                        connection_type,
                        latency_ms,
                    };

                    if accepted { Ok(model) } else { Err(model) }
                })
                .partition_result::<Vec<_>, Vec<_>, _, _>();
            active_connections.sort_by_key(|c| c.connected_at);
            pending_connections.sort_by_key(|c| c.connected_at);
            (active_connections, pending_connections)
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

            active_connections,
            pending_connections,
        }
    }
}

#[derive(Debug, Clone)]
struct Protocol {
    peer_tx: mpsc::UnboundedSender<(NodeId, PeerHandle)>,
}

impl Protocol {
    const ALPN: &'static [u8] = b"musicopy/0";

    fn new(peer_tx: mpsc::UnboundedSender<(NodeId, PeerHandle)>) -> Self {
        Self { peer_tx }
    }
}

impl ProtocolHandler for Protocol {
    fn accept(&self, connection: iroh::endpoint::Connection) -> Boxed<anyhow::Result<()>> {
        let handle_tx = self.peer_tx.clone();
        Box::pin(async move {
            // We can get the remote's node id from the connection.
            let node_id = connection.remote_node_id()?;
            println!("accepted connection from {node_id}");

            let mut peer = Peer::new(connection, handle_tx);
            peer.run().await?;

            peer.connection.closed().await;

            Ok(())
        })
    }
}

#[derive(Debug)]
enum PeerCommand {
    Accept,

    Close,
}

#[derive(Debug, Clone)]
struct PeerHandle {
    connected_at: u64,
    accepted: Arc<AtomicBool>,
    tx: mpsc::UnboundedSender<PeerCommand>,
}

/// A message sent by the client end of a connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
enum ClientMessage {
    Identify(String),
}

/// A message sent by the server end of a connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
enum ServerMessage {
    Identify(String),
    Accepted,
    Index,
}

struct Peer {
    connection: Connection,
    handle_tx: mpsc::UnboundedSender<(NodeId, PeerHandle)>,
    connected_at: u64,
    accepted: Arc<AtomicBool>,
    tx: mpsc::UnboundedSender<PeerCommand>,
    rx: mpsc::UnboundedReceiver<PeerCommand>,
}

impl Peer {
    fn new(connection: Connection, handle_tx: mpsc::UnboundedSender<(NodeId, PeerHandle)>) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        Self {
            connection,
            handle_tx,
            connected_at: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            accepted: Arc::new(AtomicBool::new(false)),
            tx,
            rx,
        }
    }

    async fn run(&mut self) -> anyhow::Result<()> {
        let remote_node_id = self.connection.remote_node_id()?;

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
                log::debug!("peer identified as {name}");
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

        // send handle to Node
        self.handle_tx
            .send((
                remote_node_id,
                PeerHandle {
                    connected_at: self.connected_at,
                    accepted: self.accepted.clone(),
                    tx: self.tx.clone(),
                },
            ))
            .expect("failed to send peer handle");

        // waiting loop
        loop {
            tokio::select! {
                Some(command) = self.rx.recv() => {
                    match command {
                        PeerCommand::Accept => {
                            // continue to next state
                            break;
                        },
                        PeerCommand::Close => {
                            self.connection.close(0u32.into(), b"close");
                            return Ok(());
                        },
                    }
                }

                Some(Ok(message)) = recv.next() => {
                    log::debug!("unexpected message (waiting): {message:?}");
                }

                else => break,
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
        send.send(ServerMessage::Index)
            .await
            .expect("failed to send Index message");

        // main loop
        loop {
            tokio::select! {
                Some(command) = self.rx.recv() => {
                    match command {
                        PeerCommand::Accept => {
                            log::warn!("unexpected Accept command in main loop");
                        },
                        PeerCommand::Close => {
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
}
