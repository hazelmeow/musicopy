use crate::clone;
use anyhow::Context;
use arc_swap::ArcSwap;
use dioxus::signals::{SyncSignal, Writable};
use futures::StreamExt;
use iroh::{protocol::Router, Endpoint, PublicKey};
use iroh_blobs::{
    format::collection::Collection,
    net_protocol::{Blobs, DownloadMode},
    rpc::client::blobs::{DownloadOptions, WrapOption},
    store::{ExportFormat, ExportMode, ImportMode, Store},
    ticket::BlobTicket,
    util::{progress::IgnoreProgressSender, SetTagOption},
    BlobFormat,
};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub struct ProtocolState {
    pub node_id: PublicKey,
    pub relay_url: String,
    pub library: Arc<Vec<String>>,
    pub ticket: Option<BlobTicket>,
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
    Download(BlobTicket, PathBuf),
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
    blobs: Blobs<iroh_blobs::store::mem::Store>,

    library: ArcSwap<Vec<String>>,
    ticket: Mutex<Option<BlobTicket>>,
}

impl ProtocolThread {
    async fn new(signal: SyncSignal<Option<ProtocolState>>) -> anyhow::Result<Arc<Self>> {
        let endpoint = Endpoint::builder().discovery_n0().bind().await?;

        let blobs = Blobs::memory().build(&endpoint);

        let router = Router::builder(endpoint)
            .accept(iroh_blobs::ALPN, blobs.clone())
            .spawn();

        let protocol = Arc::new(Self {
            signal: Mutex::new(signal),

            router,
            blobs,

            library: ArcSwap::new(Arc::new(Vec::new())),
            ticket: Mutex::new(None),
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
                        ProtocolCommand::Download(ticket, destination) => {
                            let protocol = self.clone();
                            tokio::spawn(async move {
                                if let Err(e) = protocol.download(ticket, destination).await {
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

        let library_strings = library.iter().map(|(name, _)| name.clone()).collect();
        self.library.store(Arc::new(library_strings));

        let db = self.blobs.store();

        // import all the files, using num_cpus workers, return names and temp tags
        let mut names_and_tags = futures::stream::iter(library)
            .map(|(name, path)| {
                clone!(db);
                async move {
                    let (temp_tag, file_size) = db
                        .import_file(
                            path,
                            ImportMode::TryReference,
                            BlobFormat::Raw,
                            IgnoreProgressSender::default(),
                        )
                        .await?;
                    anyhow::Ok((name, temp_tag, file_size))
                }
            })
            // .buffer_unordered(num_cpus::get()) // TODO: multithread protocol
            .buffer_unordered(1)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect::<anyhow::Result<Vec<_>>>()?;

        names_and_tags.sort_by(|(a, _, _), (b, _, _)| a.cmp(b));

        // total size
        let size = names_and_tags.iter().map(|(_, _, size)| *size).sum::<u64>();

        // collect the (name, hash) tuples into a collection
        // we must also keep the tags around so the data does not get gced.
        let (collection, tags) = names_and_tags
            .into_iter()
            .map(|(name, tag, _)| ((name, *tag.hash()), tag))
            .unzip::<_, _, Collection, Vec<_>>();
        let temp_tag = collection.clone().store(db).await?;

        // now that the collection is stored, we can drop the tags
        // data is protected by the collection
        drop(tags);

        let node_id = self.router.endpoint().node_id();
        let ticket = BlobTicket::new(node_id.into(), *temp_tag.hash(), temp_tag.format())?;

        {
            let mut ticket_mutex = self.ticket.lock().unwrap();
            *ticket_mutex = Some(ticket);
        }

        self.notify_state();

        Ok(())
    }

    async fn download(
        self: &Arc<Self>,
        ticket: BlobTicket,
        destination: PathBuf,
    ) -> anyhow::Result<()> {
        println!("downloading {ticket}");

        let blobs_client = self.blobs.client();
        let progress = blobs_client
            .download_with_opts(
                ticket.hash(),
                DownloadOptions {
                    format: BlobFormat::HashSeq,
                    nodes: vec![ticket.node_addr().clone()],
                    tag: SetTagOption::Auto,
                    mode: DownloadMode::Queued,
                },
            )
            .await?;
        progress.await?;

        blobs_client
            .export(
                ticket.hash(),
                destination,
                ExportFormat::Collection,
                ExportMode::TryReference,
            )
            .await?;

        Ok(())
    }

    fn notify_state(self: &Arc<Self>) {
        let ticket = {
            let ticket = self.ticket.lock().unwrap();
            ticket.clone()
        };

        let mut signal = self.signal.lock().unwrap();
        signal.set(Some(ProtocolState {
            node_id: self.router.endpoint().node_id(),
            relay_url: format!("{:?}", self.router.endpoint().home_relay().get()),
            library: self.library.load_full(),
            ticket,
        }));
    }
}
