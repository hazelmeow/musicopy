use anyhow::Context;
use iroh::NodeId;
use itertools::Itertools;
use rusqlite::OptionalExtension;
use std::path::Path;

pub struct Root {
    pub id: u64,
    pub node_id: NodeId,
    pub name: String,
    pub path: String,
}

pub struct File {
    pub id: u64,
    pub hash_kind: String,
    pub hash: Vec<u8>,
    pub node_id: NodeId,
    pub root: String,
    pub path: String,
    pub local_tree: String,
    pub local_path: String,
}

pub struct InsertFile<'a> {
    pub hash_kind: &'a str,
    pub hash: &'a [u8],
    pub root: &'a str,
    pub path: &'a str,
    pub local_tree: &'a str,
    pub local_path: &'a str,
}

pub struct RecentServer {
    pub node_id: NodeId,
    pub connected_at: u64,
}

#[derive(Debug)]
pub struct Database {
    conn: rusqlite::Connection,
}

impl Database {
    /// Open the database from a file.
    pub fn open_file(path: &Path) -> anyhow::Result<Self> {
        let conn = rusqlite::Connection::open(path)?;
        Self::new_from_connection(conn)
    }

    /// Open the databease in memory.
    pub fn open_in_memory() -> anyhow::Result<Self> {
        log::warn!("using in-memory database");
        let conn = rusqlite::Connection::open_in_memory()?;
        Self::new_from_connection(conn)
    }

    fn new_from_connection(conn: rusqlite::Connection) -> anyhow::Result<Self> {
        let db = Self { conn };

        db.create_tables()?;

        Ok(db)
    }

    fn create_tables(&self) -> anyhow::Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS roots (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                node_id TEXT NOT NULL,
                name TEXT NOT NULL,
                path TEXT NOT NULL,
                UNIQUE (node_id, name)
            )",
            [],
        )?;
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS files (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                hash_kind TEXT NOT NULL,
                hash BLOB NOT NULL,
                node_id TEXT NOT NULL,
                root TEXT NOT NULL,
                path TEXT NOT NULL,
                local_tree TEXT NOT NULL,
                local_path TEXT NOT NULL,
                UNIQUE (node_id, root, path)
            )",
            [],
        )?;
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS trusted_nodes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                node_id TEXT NOT NULL UNIQUE
            )",
            [],
        )?;
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS recent_servers (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                node_id TEXT NOT NULL UNIQUE,
                connected_at INTEGER NOT NULL
            )",
            [],
        )?;
        Ok(())
    }

    pub fn reset(&self) -> anyhow::Result<()> {
        self.conn.execute("DROP TABLE IF EXISTS roots", [])?;
        self.conn.execute("DROP TABLE IF EXISTS files", [])?;
        self.create_tables()?;
        Ok(())
    }

    pub fn add_root(&self, node_id: NodeId, name: &str, path: &str) -> anyhow::Result<()> {
        let node_id = node_id_to_string(&node_id);
        self.conn.execute(
            "INSERT INTO roots (node_id, name, path) VALUES (?, ?, ?)",
            [&node_id, name, path],
        )?;
        Ok(())
    }

    pub fn delete_root_by_name(&self, node_id: NodeId, name: &str) -> anyhow::Result<()> {
        let node_id = node_id_to_string(&node_id);
        self.conn.execute(
            "DELETE FROM roots WHERE node_id = ? AND name = ?",
            [&node_id, name],
        )?;
        Ok(())
    }

    pub fn get_roots_by_node_id(&self, node_id: NodeId) -> anyhow::Result<Vec<Root>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, node_id, name, path FROM roots WHERE node_id = ?")
            .expect("should prepare statement");

        let node_id = node_id_to_string(&node_id);
        stmt.query_and_then([node_id], |row| {
            let node_id =
                hex::decode(row.get::<_, String>(1)?).context("failed to parse node id")?;
            let node_id =
                NodeId::try_from(node_id.as_slice()).context("failed to parse node id")?;

            Ok(Root {
                id: row.get(0)?,
                node_id,
                name: row.get(2)?,
                path: row.get(3)?,
            })
        })
        .expect("should bind parameters")
        .collect()
    }

    pub fn count_files_by_root(&self, node_id: NodeId, root: &str) -> anyhow::Result<u64> {
        let mut stmt = self
            .conn
            .prepare("SELECT COUNT(*) FROM files WHERE node_id = ? AND root = ?")
            .expect("should prepare statement");

        let node_id = node_id_to_string(&node_id);
        let count: u64 = stmt
            .query_row([&node_id, root], |row| row.get(0))
            .context("failed to count files")?;

        Ok(count)
    }

    /// Delete all local files and insert new ones.
    pub fn replace_local_files<'a>(
        &mut self,
        local_node_id: NodeId,
        iter: impl Iterator<Item = InsertFile<'a>>,
    ) -> anyhow::Result<()> {
        let tx = self
            .conn
            .transaction()
            .context("failed to begin transaction")?;

        tx.execute(
            "DELETE FROM files WHERE node_id = ?",
            [node_id_to_string(&local_node_id)],
        )?;

        {
            let mut stmt = tx.prepare("INSERT INTO files (hash_kind, hash, node_id, root, path, local_tree, local_path) VALUES (?, ?, ?, ?, ?, ?, ?)")?;
            for file in iter {
                stmt.execute((
                    file.hash_kind,
                    file.hash,
                    node_id_to_string(&local_node_id),
                    file.root,
                    file.path,
                    file.local_tree,
                    file.local_path,
                ))?;
            }
        }

        tx.commit().context("failed to commit transaction")?;

        Ok(())
    }

    /// Insert a file from a remote node, updating the existing entry if it exists.
    pub fn insert_remote_file<'a>(
        &mut self,
        remote_node_id: NodeId,
        file: InsertFile<'a>,
    ) -> anyhow::Result<()> {
        let mut stmt = self.conn.prepare(
            "INSERT INTO files (hash_kind, hash, node_id, root, path, local_tree, local_path) VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(node_id, root, path) DO UPDATE SET hash_kind = excluded.hash_kind, hash = excluded.hash, local_tree = excluded.local_tree, local_path = excluded.local_path"
        )?;

        stmt.execute((
            file.hash_kind,
            file.hash,
            node_id_to_string(&remote_node_id),
            file.root,
            file.path,
            file.local_tree,
            file.local_path,
        ))?;

        Ok(())
    }

    pub fn get_files(&self) -> anyhow::Result<Vec<File>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, hash_kind, hash, node_id, root, path, local_tree, local_path FROM files")
            .expect("should prepare statement");

        stmt.query_and_then([], |row| {
            let node_id =
                hex::decode(row.get::<_, String>(3)?).context("failed to parse node id")?;
            let node_id =
                NodeId::try_from(node_id.as_slice()).context("failed to parse node id")?;

            Ok(File {
                id: row.get(0)?,
                hash_kind: row.get(1)?,
                hash: row.get(2)?,
                node_id,
                root: row.get(4)?,
                path: row.get(5)?,
                local_tree: row.get(6)?,
                local_path: row.get(7)?,
            })
        })
        .expect("should bind parameters")
        .collect()
    }

    /// Get files where node ID is the given node ID.
    pub fn get_files_by_node_id(&self, node_id: NodeId) -> anyhow::Result<Vec<File>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, hash_kind, hash, node_id, root, path, local_tree, local_path FROM files WHERE node_id = ?")
            .expect("should prepare statement");

        let node_id = node_id_to_string(&node_id);
        stmt.query_and_then([&node_id], |row| {
            let node_id =
                hex::decode(row.get::<_, String>(3)?).context("failed to parse node id")?;
            let node_id =
                NodeId::try_from(node_id.as_slice()).context("failed to parse node id")?;

            Ok(File {
                id: row.get(0)?,
                hash_kind: row.get(1)?,
                hash: row.get(2)?,
                node_id,
                root: row.get(4)?,
                path: row.get(5)?,
                local_tree: row.get(6)?,
                local_path: row.get(7)?,
            })
        })
        .expect("should bind parameters")
        .collect()
    }

    /// Get files where node ID is not the given node ID.
    pub fn get_files_by_ne_node_id(&self, node_id: NodeId) -> anyhow::Result<Vec<File>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, hash_kind, hash, node_id, root, path, local_tree, local_path FROM files WHERE node_id != ?")
            .expect("should prepare statement");

        let node_id = node_id_to_string(&node_id);
        stmt.query_and_then([&node_id], |row| {
            let node_id =
                hex::decode(row.get::<_, String>(3)?).context("failed to parse node id")?;
            let node_id =
                NodeId::try_from(node_id.as_slice()).context("failed to parse node id")?;

            Ok(File {
                id: row.get(0)?,
                hash_kind: row.get(1)?,
                hash: row.get(2)?,
                node_id,
                root: row.get(4)?,
                path: row.get(5)?,
                local_tree: row.get(6)?,
                local_path: row.get(7)?,
            })
        })
        .expect("should bind parameters")
        .collect()
    }

    pub fn exists_file_by_node_root_path(
        &self,
        node_id: NodeId,
        root: &str,
        path: &str,
    ) -> anyhow::Result<bool> {
        let mut stmt = self
            .conn
            .prepare("SELECT 1 FROM files WHERE node_id = ? AND root = ? AND path = ? LIMIT 1")
            .expect("should prepare statement");

        let node_id = node_id_to_string(&node_id);
        let exists: Option<u8> = stmt
            .query_row([&node_id, root, path], |row| row.get(0))
            .optional()
            .context("failed to query row")?;

        Ok(exists.is_some())
    }

    pub fn get_file_by_node_root_path(
        &self,
        node_id: NodeId,
        root: &str,
        path: &str,
    ) -> anyhow::Result<Option<File>> {
        let mut stmt = self
        .conn
        .prepare("SELECT id, hash_kind, hash, node_id, root, path, local_tree, local_path FROM files WHERE node_id = ? AND root = ? AND path = ?")
        .expect("should prepare statement");

        let node_id = node_id_to_string(&node_id);
        stmt.query_and_then([&node_id, root, path], |row| {
            let node_id =
                hex::decode(row.get::<_, String>(3)?).context("failed to parse node id")?;
            let node_id =
                NodeId::try_from(node_id.as_slice()).context("failed to parse node id")?;

            Ok(File {
                id: row.get(0)?,
                hash_kind: row.get(1)?,
                hash: row.get(2)?,
                node_id,
                root: row.get(4)?,
                path: row.get(5)?,
                local_tree: row.get(6)?,
                local_path: row.get(7)?,
            })
        })
        .expect("should bind parameters")
        .next()
        .transpose()
    }

    pub fn get_files_by_node_root_path(
        &self,
        keys: impl ExactSizeIterator<Item = (NodeId, String, String)>,
    ) -> anyhow::Result<Vec<File>> {
        if keys.len() == 0 {
            return Ok(Vec::new());
        }

        let placeholders = std::iter::repeat_n("(?, ?, ?)", keys.len()).join(", ");
        let sql = format!(
            "SELECT id, hash_kind, hash, node_id, root, path, local_tree, local_path FROM files WHERE (node_id, root, path) IN ({placeholders})"
        );

        let mut stmt = self.conn.prepare(&sql).expect("should prepare statement");

        let params_flat = rusqlite::params_from_iter(keys.flat_map(|(node_id, root, path)| {
            let node_id_string = node_id_to_string(&node_id);
            [node_id_string, root, path]
        }));

        stmt.query_and_then(params_flat, |row| {
            let node_id =
                hex::decode(row.get::<_, String>(3)?).context("failed to parse node id")?;
            let node_id =
                NodeId::try_from(node_id.as_slice()).context("failed to parse node id")?;

            Ok(File {
                id: row.get(0)?,
                hash_kind: row.get(1)?,
                hash: row.get(2)?,
                node_id,
                root: row.get(4)?,
                path: row.get(5)?,
                local_tree: row.get(6)?,
                local_path: row.get(7)?,
            })
        })
        .expect("should bind parameters")
        .collect()
    }

    pub fn remove_files_by_node_root_path(
        &self,
        keys: impl ExactSizeIterator<Item = (NodeId, String, String)>,
    ) -> anyhow::Result<()> {
        if keys.len() == 0 {
            return Ok(());
        }

        let placeholders = std::iter::repeat_n("(?, ?, ?)", keys.len()).join(", ");
        let sql = format!("DELETE FROM files WHERE (node_id, root, path) IN ({placeholders})");

        let mut stmt = self.conn.prepare(&sql).expect("should prepare statement");

        let params_flat = rusqlite::params_from_iter(keys.flat_map(|(node_id, root, path)| {
            let node_id_string = node_id_to_string(&node_id);
            [node_id_string, root, path]
        }));

        stmt.execute(params_flat)?;

        Ok(())
    }

    pub fn get_trusted_nodes(&self) -> anyhow::Result<Vec<NodeId>> {
        let mut stmt = self
            .conn
            .prepare("SELECT node_id FROM trusted_nodes")
            .expect("should prepare statement");

        stmt.query_and_then([], |row| {
            let node_id =
                hex::decode(row.get::<_, String>(0)?).context("failed to parse node id")?;
            NodeId::try_from(node_id.as_slice()).context("failed to parse node id")
        })
        .expect("should bind parameters")
        .collect()
    }

    pub fn add_trusted_node(&self, node_id: NodeId) -> anyhow::Result<()> {
        let node_id = node_id_to_string(&node_id);
        self.conn.execute(
            "INSERT INTO trusted_nodes (node_id) VALUES (?) ON CONFLICT(node_id) DO NOTHING",
            [&node_id],
        )?;
        Ok(())
    }

    pub fn remove_trusted_node(&self, node_id: NodeId) -> anyhow::Result<()> {
        let node_id = node_id_to_string(&node_id);
        self.conn
            .execute("DELETE FROM trusted_nodes WHERE node_id = ?", [&node_id])?;
        Ok(())
    }

    pub fn is_node_trusted(&self, node_id: NodeId) -> anyhow::Result<bool> {
        let mut stmt = self
            .conn
            .prepare("SELECT 1 FROM trusted_nodes WHERE node_id = ? LIMIT 1")
            .expect("should prepare statement");

        let node_id = node_id_to_string(&node_id);
        let exists: Option<u8> = stmt
            .query_row([&node_id], |row| row.get(0))
            .optional()
            .context("failed to query trusted node")?;

        Ok(exists.is_some())
    }

    pub fn update_recent_server(&self, node_id: NodeId, connected_at: u64) -> anyhow::Result<()> {
        let node_id = node_id_to_string(&node_id);
        self.conn.execute(
            "INSERT INTO recent_servers (node_id, connected_at) VALUES (?, ?)
            ON CONFLICT(node_id) DO UPDATE SET connected_at = excluded.connected_at",
            [&node_id, &connected_at.to_string()],
        )?;
        Ok(())
    }

    pub fn get_recent_servers(&self) -> anyhow::Result<Vec<RecentServer>> {
        let mut stmt = self
            .conn
            .prepare("SELECT node_id, connected_at FROM recent_servers ORDER BY connected_at DESC")
            .expect("should prepare statement");

        stmt.query_and_then([], |row| {
            let node_id =
                hex::decode(row.get::<_, String>(0)?).context("failed to parse node id")?;
            let node_id =
                NodeId::try_from(node_id.as_slice()).context("failed to parse node id")?;
            let connected_at = row.get::<_, u64>(1)?;
            Ok(RecentServer {
                node_id,
                connected_at,
            })
        })
        .expect("should bind parameters")
        .collect()
    }
}

fn node_id_to_string(node_id: &NodeId) -> String {
    hex::encode(node_id)
}
