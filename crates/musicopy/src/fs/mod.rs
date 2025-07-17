//! # Filesystem module
//! Cross-platform filesystem abstraction, mainly to support Android's document system.
//!
//! Notes:
//! - Lots of operations on Android require the URI of the tree containing a document
//! - For now we're representing paths as a combination of a tree URI and a subpath.
//! - URIs on Android are opaque, they can't be treated as paths
//!     - Need to recursively traverse using JNI calls, will provide helpers
//! - content:// urls on android, maybe file:// urls on desktop
//! - For now we're not returning URIs anywhere, mostly just files.
//!     - We might want a notion of resolved vs unresolved, so we can return a URI to a document
//!       in a deep subfolder instead of its path to not have to resolve it again.
//! - Will add support for iOS/MacOS sandboxing ("bookmarks"?) later

#[cfg(target_os = "android")]
mod android;

use std::{path::PathBuf, pin::Pin};
use tokio::{
    fs::{File as TokioFile, OpenOptions},
    io::{AsyncRead, AsyncWrite, AsyncWriteExt},
};

pub enum OpenMode {
    Read,
    Write,
}

/// Struct representing a path in a subtree of the filesystem.
///
/// This is required for Android, where filesystem access is granted to
/// specific subtrees, and requires the tree URI to perform operations.
///
/// Outside of Android, the standard filesystem path can be obtained by
/// appending the path to the tree.
#[derive(Debug, Clone)]
pub struct TreePath {
    /// The URI of the tree.
    ///
    /// On Android, this is a tree URI.
    ///
    /// Otherwise, this is... a path? or a file uri? TODO
    tree: String,
    /// The subpath within the tree.
    path: PathBuf,
}

impl TreePath {
    pub fn new(root: String, path: PathBuf) -> Self {
        Self { tree: root, path }
    }

    pub fn from_root(root: String) -> Self {
        Self {
            tree: root,
            path: PathBuf::new(),
        }
    }

    pub fn push(&mut self, component: &str) {
        self.path.push(component);
    }

    pub fn join(&self, component: &str) -> Self {
        let mut new_path = self.path.clone();
        new_path.push(component);
        Self {
            tree: self.tree.clone(),
            path: new_path,
        }
    }

    pub fn parent(&self) -> Option<Self> {
        self.path.parent().map(|p| Self {
            tree: self.tree.clone(),
            path: p.to_path_buf(),
        })
    }

    pub fn is_empty(&self) -> bool {
        self.path.as_os_str().is_empty()
    }

    #[cfg(not(target_os = "android"))]
    pub fn resolve_path(&self) -> PathBuf {
        let mut p = PathBuf::from(&self.tree);
        p.push(&self.path);
        p
    }
}

pub struct TreeFile {
    #[cfg(not(target_os = "android"))]
    file: TokioFile,

    #[cfg(target_os = "android")]
    file: android::FileHandle,
}

impl TreeFile {
    pub async fn create(path: &TreePath) -> anyhow::Result<Self> {
        #[cfg(not(target_os = "android"))]
        {
            let resolved_path = path.resolve_path();
            let file = TokioFile::create(&resolved_path).await?;
            Ok(Self { file })
        }
        #[cfg(target_os = "android")]
        {
            let file =
                android::open_or_create_file(path, android::AccessMode::Create, OpenMode::Write)?;
            Ok(Self { file })
        }
    }

    pub async fn open(path: &TreePath, mode: OpenMode) -> anyhow::Result<Self> {
        #[cfg(not(target_os = "android"))]
        {
            let resolved_path = path.resolve_path();
            let file = match mode {
                OpenMode::Read => OpenOptions::new().read(true).open(&resolved_path).await?,
                OpenMode::Write => {
                    OpenOptions::new()
                        .write(true)
                        .truncate(true)
                        .open(&resolved_path)
                        .await?
                }
            };
            Ok(Self { file })
        }
        #[cfg(target_os = "android")]
        {
            let file = android::open_or_create_file(path, android::AccessMode::Open, mode)?;
            Ok(Self { file })
        }
    }

    pub async fn open_or_create(path: &TreePath, mode: OpenMode) -> anyhow::Result<Self> {
        #[cfg(not(target_os = "android"))]
        {
            let resolved_path = path.resolve_path();
            let file = match mode {
                OpenMode::Read => {
                    OpenOptions::new()
                        .read(true)
                        .truncate(false)
                        .create(true)
                        .open(&resolved_path)
                        .await?
                }
                OpenMode::Write => {
                    OpenOptions::new()
                        .write(true)
                        .truncate(true)
                        .create(true)
                        .open(&resolved_path)
                        .await?
                }
            };
            Ok(Self { file })
        }
        #[cfg(target_os = "android")]
        {
            let file = android::open_or_create_file(path, android::AccessMode::OpenOrCreate, mode)?;
            Ok(Self { file })
        }
    }

    fn file(&self) -> &TokioFile {
        #[cfg(not(target_os = "android"))]
        {
            &self.file
        }
        #[cfg(target_os = "android")]
        {
            self.file.file()
        }
    }

    fn file_mut(&mut self) -> &mut TokioFile {
        #[cfg(not(target_os = "android"))]
        {
            &mut self.file
        }
        #[cfg(target_os = "android")]
        {
            self.file.file_mut()
        }
    }

    pub async fn write_all(&mut self, buf: &[u8]) -> anyhow::Result<()> {
        self.file_mut().write_all(buf).await?;
        Ok(())
    }
}

impl AsyncRead for TreeFile {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        Pin::new(self.file_mut()).poll_read(cx, buf)
    }
}

impl AsyncWrite for TreeFile {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        Pin::new(self.file_mut()).poll_write(cx, buf)
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        Pin::new(self.file_mut()).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        Pin::new(self.file_mut()).poll_shutdown(cx)
    }

    fn poll_write_vectored(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        bufs: &[std::io::IoSlice<'_>],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        Pin::new(self.file_mut()).poll_write_vectored(cx, bufs)
    }

    fn is_write_vectored(&self) -> bool {
        self.file().is_write_vectored()
    }
}

pub async fn create_dir_all(path: &TreePath) -> anyhow::Result<()> {
    #[cfg(not(target_os = "android"))]
    {
        let resolved_path = path.resolve_path();
        tokio::fs::create_dir_all(&resolved_path).await?;
        Ok(())
    }
    #[cfg(target_os = "android")]
    {
        android::create_dir_all(path)?;
        Ok(())
    }
}
