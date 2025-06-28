use iroh::{
    Endpoint, NodeAddr,
    protocol::{ProtocolHandler, Router},
};
use n0_future::future::Boxed;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::mpsc;

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
}

#[derive(Debug)]
pub enum NodeCommand {
    Send(NodeAddr, String),
    Stop,
}

#[derive(Debug)]
pub struct Node {
    router: Router,
    protocol: Protocol,
}

impl Node {
    pub async fn new() -> anyhow::Result<Arc<Self>> {
        let endpoint = Endpoint::builder().discovery_n0().bind().await?;
        let protocol = Protocol::new();

        let router = Router::builder(endpoint)
            .accept(Protocol::ALPN, protocol.clone())
            .spawn();

        Ok(Arc::new(Self { router, protocol }))
    }

    pub async fn run(
        self: &Arc<Self>,
        mut rx: mpsc::UnboundedReceiver<NodeCommand>,
    ) -> anyhow::Result<()> {
        loop {
            tokio::select! {
                Some(command) = rx.recv() => {
                    match command {
                        NodeCommand::Send(addr, text) => {
                            let this = self.clone();
                            tokio::task::spawn(async move {
                                // TODO: return error
                                let _ = this.send(addr, text).await;
                            });
                        },
                        NodeCommand::Stop => break,
                    }
                }
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        let _ = self.router.shutdown().await;

        Ok(())
    }

    pub async fn send(self: &Arc<Self>, addr: NodeAddr, text: String) -> anyhow::Result<()> {
        // Open a connection to the accepting node
        let conn = self.router.endpoint().connect(addr, Protocol::ALPN).await?;

        // Open a bidirectional QUIC stream
        let (mut send, mut recv) = conn.open_bi().await?;

        // Send some data to be echoed
        send.write_all(b"Hello, world!").await?;

        // Signal the end of data for this particular stream
        send.finish()?;

        // Receive the echo, but limit reading up to maximum 1000 bytes
        let response = recv.read_to_end(1000).await?;
        assert_eq!(&response, b"Hello, world!");

        // Explicitly close the whole connection.
        conn.close(0u32.into(), b"bye!");

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
        }
    }
}

#[derive(Debug, Clone)]
pub struct Protocol {}

impl Protocol {
    pub const ALPN: &[u8] = b"iroh-compose-demo/0";

    pub fn new() -> Self {
        Self {}
    }
}

impl Default for Protocol {
    fn default() -> Self {
        Self::new()
    }
}

impl ProtocolHandler for Protocol {
    fn accept(&self, connection: iroh::endpoint::Connection) -> Boxed<anyhow::Result<()>> {
        Box::pin(async move {
            // We can get the remote's node id from the connection.
            let node_id = connection.remote_node_id()?;
            println!("accepted connection from {node_id}");

            // Our protocol is a simple request-response protocol, so we expect the
            // connecting peer to open a single bi-directional stream.
            let (mut send, mut recv) = connection.accept_bi().await?;

            // Echo any bytes received back directly.
            // This will keep copying until the sender signals the end of data on the stream.
            let bytes_sent = tokio::io::copy(&mut recv, &mut send).await?;
            println!("Copied over {bytes_sent} byte(s)");

            // By calling `finish` on the send stream we signal that we will not send anything
            // further, which makes the receive stream on the other end terminate.
            send.finish()?;

            // Wait until the remote closes the connection, which it does once it
            // received the response.
            connection.closed().await;

            Ok(())
        })
    }
}
