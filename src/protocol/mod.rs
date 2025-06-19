use anyhow::Context;
use arc_swap::ArcSwap;
use dioxus::signals::{SyncSignal, Writable};
use iroh::{
    protocol::{ProtocolHandler, Router},
    Endpoint, NodeAddr, PublicKey,
};
use n0_future::future::Boxed;
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::mpsc,
};

#[derive(Debug, Clone)]
pub struct ProtocolState {
    pub node_id: PublicKey,
    pub relay_url: String,
    pub library: Arc<Vec<String>>,
}

/// Cloneable handle to the protocol.
#[derive(Debug, Clone)]
pub struct ProtocolHandle {
    tx: mpsc::UnboundedSender<ProtocolCommand>,
}

impl ProtocolHandle {
    pub fn send(&self, command: ProtocolCommand) {
        let _ = self.tx.send(command);
    }
}

#[derive(Debug, Clone)]
pub enum ProtocolCommand {
    Scan(Vec<String>),
    Download(NodeAddr, PathBuf),
    Shutdown,
}

pub fn start_node(signal: SyncSignal<Option<ProtocolState>>) -> ProtocolHandle {
    let (tx, rx) = mpsc::unbounded_channel();

    std::thread::spawn(move || {
        let builder = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("should build runtime");

        builder.block_on(async move {
            let protocol_thread = match ProtocolThread::new(signal).await {
                Ok(x) => x,
                Err(e) => {
                    println!("error creating ProtocolThread: {e:#}");
                    return;
                }
            };

            if let Err(e) = protocol_thread.run(rx).await {
                println!("error running ProtocolThread: {e:#}");
            }
        });
    });

    ProtocolHandle { tx }
}

#[derive(Debug)]
struct ProtocolThread {
    signal: Mutex<SyncSignal<Option<ProtocolState>>>,

    router: Router,
    protocol: Protocol,
    library: Arc<ArcSwap<Vec<String>>>,
}

impl ProtocolThread {
    async fn new(signal: SyncSignal<Option<ProtocolState>>) -> anyhow::Result<Arc<Self>> {
        let endpoint = Endpoint::builder().discovery_n0().bind().await?;

        let library = Arc::new(ArcSwap::new(Arc::new(Vec::new())));

        let protocol = Protocol::new(library.clone());

        let router = Router::builder(endpoint)
            .accept(Protocol::ALPN, protocol.clone())
            .spawn();

        let protocol = Arc::new(Self {
            signal: Mutex::new(signal),

            router,
            protocol,

            library,
        });
        protocol.notify_state();

        Ok(protocol)
    }

    async fn run(
        self: &Arc<Self>,
        mut rx: mpsc::UnboundedReceiver<ProtocolCommand>,
    ) -> anyhow::Result<()> {
        loop {
            tokio::select! {
                Some(command) = rx.recv() => {
                    match command {
                        ProtocolCommand::Scan(paths) => {
                            let protocol = self.clone();
                            tokio::spawn(async move {
                                if let Err(e) = protocol.scan(paths).await {
                                    println!("error scanning library: {e:#}");
                                }
                            });
                        }
                        ProtocolCommand::Download(addr, destination) => {
                            let protocol = self.clone();
                            tokio::spawn(async move {
                                if let Err(e) = protocol.download(addr, destination).await {
                                    println!("error downloading: {e:#}");
                                }
                            });
                        }
                        ProtocolCommand::Shutdown => {
                            break;
                        }
                    }
                }
            }
        }

        self.router.shutdown().await?;

        Ok(())
    }

    async fn scan(self: &Arc<Self>, paths: Vec<String>) -> anyhow::Result<()> {
        let mut library = Vec::new();

        for root in paths {
            let root = root.parse::<PathBuf>().context("failed to parse path")?;

            anyhow::ensure!(root.exists(), "root {} does not exist", root.display());

            let walker =
                globwalk::GlobWalkerBuilder::new(&root, "*.{mp3,flac,ogg,m4a,wav,aif,aiff}")
                    .build()
                    .expect("glob shouldn't fail")
                    .filter_map(Result::ok);

            for entry in walker {
                if !entry.file_type().is_file() {
                    continue;
                }

                let path = entry.into_path();

                let name = path.strip_prefix(&root)?.to_string_lossy().into_owned();

                library.push((name, path));
            }
        }

        // let library_strings = library.iter().map(|(name, _)| name.clone()).collect();
        let library_strings = library
            .iter()
            .map(|(_, path)| path.to_string_lossy().to_string())
            .collect();
        self.library.store(Arc::new(library_strings));

        self.notify_state();

        Ok(())
    }

    async fn download(
        self: &Arc<Self>,
        addr: NodeAddr,
        destination: PathBuf,
    ) -> anyhow::Result<()> {
        println!("downloading from {addr:?}");

        // connect to the remote node
        let conn = self.router.endpoint().connect(addr, Protocol::ALPN).await?;

        // accept a unidirectional stream
        let mut recv = conn.accept_uni().await?;

        // read index message
        let index_message_len = recv.read_u32().await?;
        let mut index_message_buf = vec![0; index_message_len as usize];
        recv.read_exact(&mut index_message_buf)
            .await
            .context("failed to read index message")?;
        let index_message: IndexMessage = postcard::from_bytes(&index_message_buf)
            .context("failed to deserialize index message")?;

        println!(
            "received index message with {} files",
            index_message.files.len()
        );

        // TODO: concurrent
        for file in index_message.files {
            println!("downloading file: {file}");

            // open a bidirectional stream to send DownloadRequest
            let (mut send, mut recv) = conn.open_bi().await?;
            let download_request = DownloadRequest { file: file.clone() };
            let download_request_buf = postcard::to_stdvec(&download_request)
                .context("failed to serialize download request")?;
            send.write_u32(download_request_buf.len() as u32)
                .await
                .context("failed to write download request length")?;
            send.write_all(&download_request_buf)
                .await
                .context("failed to write download request")?;

            println!("sent download request for {file}");

            // receive file content
            let file_content_len = recv.read_u32().await?;
            let mut file_content_buf = vec![0; file_content_len as usize];
            recv.read_exact(&mut file_content_buf)
                .await
                .context("failed to read file content")?;

            let filename = file
                .split('/')
                .next_back()
                .ok_or_else(|| anyhow::anyhow!("failed to extract filename from path"))?
                .to_string();

            let file_path = destination.join(&filename);
            if let Some(parent) = file_path.parent() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .context("failed to create parent directory")?;
            }

            tokio::fs::write(&file_path, file_content_buf)
                .await
                .context("failed to write file content")?;

            println!("saved file to {}", file_path.display());
        }

        // explicitly close the whole connection
        conn.close(0u32.into(), b"bye!");

        Ok(())
    }

    fn notify_state(self: &Arc<Self>) {
        let mut signal = self.signal.lock().unwrap();
        signal.set(Some(ProtocolState {
            node_id: self.router.endpoint().node_id(),
            relay_url: format!("{:?}", self.router.endpoint().home_relay().get()),
            library: self.library.load_full(),
        }));
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexMessage {
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadRequest {
    pub file: String,
}

#[derive(Debug, Clone)]
pub struct Protocol {
    library: Arc<ArcSwap<Vec<String>>>,
}

impl Protocol {
    pub const ALPN: &[u8] = b"musicopy/0";

    pub fn new(library: Arc<ArcSwap<Vec<String>>>) -> Self {
        Self { library }
    }
}

impl ProtocolHandler for Protocol {
    fn accept(&self, connection: iroh::endpoint::Connection) -> Boxed<anyhow::Result<()>> {
        let library = self.library.load_full();

        Box::pin(async move {
            // We can get the remote's node id from the connection.
            let node_id = connection.remote_node_id()?;
            println!("accepted connection from {node_id}");

            // open a unidirectional stream to send IndexMessage
            let mut send = connection.open_uni().await?;

            let index_message = IndexMessage {
                files: library.to_vec(),
            };
            let index_message_buf =
                postcard::to_stdvec(&index_message).context("failed to serialize index message")?;
            send.write_u32(index_message_buf.len() as u32)
                .await
                .context("failed to write index message length")?;
            send.write_all(&index_message_buf)
                .await
                .context("failed to write index message")?;

            loop {
                tokio::select! {
                    _ = connection.closed() => {
                        println!("connection closed");
                        break;
                    }

                    accept_result = connection.accept_bi() => {
                        match accept_result {
                            Ok((mut send, mut recv)) => {
                                // receive download request
                                let download_req_len = recv.read_u32().await?;
                                let mut download_req_buf = vec![0; download_req_len as usize];
                                recv
                                    .read_exact(&mut download_req_buf)
                                    .await
                                    .context("failed to read download request")?;
                                let download_req: DownloadRequest =
                                    postcard::from_bytes(&download_req_buf).context("failed to deserialize download request")?;

                                println!("received download request for {}", download_req.file);

                                if let Some(file) = library.iter().find(|f| f == &&download_req.file) {
                                    // send file content
                                    let file_path = PathBuf::from(file);
                                    if file_path.exists() {
                                        let file_content = tokio::fs::read(file_path).await?;
                                        send.write_u32(file_content.len() as u32).await?;
                                        send.write_all(&file_content).await?;
                                    } else {
                                        anyhow::bail!("file not found")
                                    }
                                } else {
                                    anyhow::bail!("file not found in index")
                                };

                                println!("finished sending file content for {}", download_req.file);
                            }

                            Err(e) => {
                                println!("accept_bi error: {e}");
                                break;
                            }
                        }
                    }
                }
            }

            println!("connection to {node_id} finished");

            Ok(())
        })
    }
}
