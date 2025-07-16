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

use std::{io::Write, path::PathBuf};

#[cfg(target_os = "android")]
mod android;

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
}

pub struct TreeFile {
    #[cfg(not(target_os = "android"))]
    file: std::fs::File,

    #[cfg(target_os = "android")]
    file: android::FileHandle,
}

impl TreeFile {
    pub fn create(path: &TreePath) -> anyhow::Result<Self> {
        #[cfg(not(target_os = "android"))]
        {
            let file = std::fs::File::create(&path.path)?;
            Ok(Self { file })
        }
        #[cfg(target_os = "android")]
        {
            let file = android::create_file(path)?;
            Ok(Self { file })
        }
    }

    pub fn open(path: &TreePath, mode: OpenMode) -> anyhow::Result<Self> {
        #[cfg(not(target_os = "android"))]
        {
            let file = match mode {
                OpenMode::Read => std::fs::OpenOptions::new().read(true).open(&path.path)?,
                OpenMode::Write => std::fs::OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .open(&path.path)?,
            };
            Ok(Self { file })
        }
        #[cfg(target_os = "android")]
        {
            let file = android::open_file(path, mode)?;
            Ok(Self { file })
        }
    }

    fn file(&self) -> &std::fs::File {
        #[cfg(not(target_os = "android"))]
        {
            &self.file
        }
        #[cfg(target_os = "android")]
        {
            self.file.file()
        }
    }

    pub fn write_all(&mut self, buf: &[u8]) -> anyhow::Result<()> {
        self.file().write_all(buf)?;
        Ok(())
    }
}

pub fn create_dir_all(path: &TreePath) -> anyhow::Result<()> {
    #[cfg(not(target_os = "android"))]
    {
        std::fs::create_dir_all(&path.path)?;
        Ok(())
    }
    #[cfg(target_os = "android")]
    {
        android::create_dir_all(path)?;
        Ok(())
    }
}
