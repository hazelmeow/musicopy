pub(crate) mod database;

use crate::protocol::database::Database;
use anyhow::Context;
use arc_swap::ArcSwap;
use dioxus::signals::{SyncSignal, Writable};
use iroh::{
    protocol::{ProtocolHandler, Router},
    Endpoint, NodeAddr, PublicKey,
};
use itertools::Itertools;
use n0_future::future::Boxed;
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
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
    pub local_roots: Vec<String>,
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
    AddRoots(Vec<String>),
    RemoveRoot(String),
    Rescan,

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

    db: Mutex<Database>,
}

impl ProtocolThread {
    async fn new(signal: SyncSignal<Option<ProtocolState>>) -> anyhow::Result<Arc<Self>> {
        let db = Mutex::new(Database::open("musicopy.db").context("failed to open database")?);

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

            db,
        });
        protocol.notify_state();

        Ok(protocol)
    }

    async fn run(
        self: &Arc<Self>,
        mut rx: mpsc::UnboundedReceiver<ProtocolCommand>,
    ) -> anyhow::Result<()> {
        self.spawn_scan();

        loop {
            tokio::select! {
                Some(command) = rx.recv() => {
                    match command {
                        ProtocolCommand::AddRoots(roots) => {
                            {
                                let db = self.db.lock().unwrap();
                                for root in roots {
                                    let path = PathBuf::from(root);
                                    let path = path.canonicalize().context("failed to canonicalize path")?;
                                    db.add_local_root(&path.to_string_lossy()).context("failed to add local root")?;
                                }
                            }

                            self.notify_state();

                            // rescan the library after adding roots
                            self.spawn_scan();
                        }
                        ProtocolCommand::RemoveRoot(path) => {
                            let path = PathBuf::from(path);
                            let path = path.canonicalize().context("failed to canonicalize path")?;
                            {
                                let db = self.db.lock().unwrap();
                                db.remove_local_root(&path.to_string_lossy()).context("failed to remove local root")?;
                            }

                            // TODO: remove files from root

                            self.notify_state();
                        }
                        ProtocolCommand::Rescan => {
                            self.spawn_scan();
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

    fn spawn_scan(self: &Arc<Self>) {
        let protocol = self.clone();
        tokio::spawn(async move {
            if let Err(e) = protocol.scan().await {
                println!("error scanning library: {e:#}");
            }
        });
    }

    // TODO: stream results asynchronously? scanning the fs is fast but transcoding is slow,
    // so when do we do that?
    async fn scan(self: &Arc<Self>) -> anyhow::Result<()> {
        // TODO: lock so only one scan is running at a time

        let mut errors = Vec::new();

        let (roots, prev_local_files) = {
            let db = self.db.lock().unwrap();
            let roots = db.get_local_roots().context("failed to get local roots")?;
            let local_files = db.get_local_files().context("failed to get local files")?;
            (roots, local_files)
        };

        // remove roots that don't exist
        let roots = roots
            .into_iter()
            .filter_map(|root| {
                let path = PathBuf::from(root);
                if path.exists() {
                    Some(path)
                } else {
                    errors.push(anyhow::anyhow!(
                        "root path `{}` does not exist",
                        path.display()
                    ));
                    None
                }
            })
            .collect::<Vec<_>>();

        // walk roots and collect entries
        let (entries, walk_errors): (Vec<_>, Vec<_>) = roots
            .iter()
            .flat_map(|root_path| {
                let walker = globwalk::GlobWalkerBuilder::new(
                    root_path,
                    "*.{mp3,flac,ogg,m4a,wav,aif,aiff}",
                )
                .file_type(globwalk::FileType::FILE)
                .build()
                .expect("glob shouldn't fail");

                walker.into_iter().map_ok(move |entry| (root_path, entry))
            })
            .partition_result();

        // extend errors
        errors.extend(
            walk_errors
                .into_iter()
                .map(|e| anyhow::anyhow!("failed to scan file {:?}: {}", e.path(), e)),
        );

        struct ScanItem<'a> {
            root: &'a Path,
            path: String,
        }

        let (local_files, scan_errors): (Vec<_>, Vec<_>) = entries
            .into_iter()
            .map(|(root_path, entry)| {
                // get path without root
                let path = entry
                    .into_path()
                    .strip_prefix(root_path)
                    .context("failed to strip root path prefix")?
                    .to_string_lossy()
                    .to_string();

                anyhow::Result::Ok(ScanItem {
                    // hash_kind: "sha256".to_string(),
                    // hash: "".to_string(),
                    root: root_path,
                    path,
                })
            })
            .partition_result();

        // extend errors
        errors.extend(
            scan_errors
                .into_iter()
                .map(|e: anyhow::Error| e.context("failed to scan file")),
        );

        let index = local_files
            .iter()
            .map(|item| {
                // TODO
                let full_path = format!("{}/{}", item.root.to_string_lossy(), item.path);
                full_path
            })
            .collect::<Vec<String>>();

        self.library.store(Arc::new(index));

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
        let local_roots = {
            let db = self.db.lock().unwrap();
            db.get_local_roots()
                .context("failed to get local roots")
                .unwrap_or_else(|_| Vec::new()) // TODO
        };

        let mut signal = self.signal.lock().unwrap();
        signal.set(Some(ProtocolState {
            node_id: self.router.endpoint().node_id(),
            relay_url: format!("{:?}", self.router.endpoint().home_relay().get()),
            local_roots,
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
