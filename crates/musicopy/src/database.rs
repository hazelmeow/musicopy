use anyhow::Context;
use iroh::NodeId;
use std::path::Path;

pub struct Root {
    pub id: u64,
    pub node_id: NodeId,
    pub name: String,
    pub path: String,
}

pub struct LocalFile {
    pub id: u64,
    pub hash_kind: String,
    pub hash: String,
    pub node_id: NodeId,
    pub root: String,
    pub path: String,
    pub local_path: String,
}

pub struct InsertFile<'a> {
    pub hash_kind: &'a str,
    pub hash: &'a str,
    pub node_id: NodeId,
    pub root: &'a str,
    pub path: &'a str,
    pub local_path: &'a str,
}

#[derive(Debug)]
pub struct Database {
    conn: rusqlite::Connection,
}

impl Database {
    pub fn open_file(path: &Path) -> anyhow::Result<Self> {
        let conn = rusqlite::Connection::open(path)?;
        Self::init_from_connection(conn)
    }

    pub fn open_in_memory() -> anyhow::Result<Self> {
        log::warn!("using in-memory database");
        let conn = rusqlite::Connection::open_in_memory()?;
        Self::init_from_connection(conn)
    }

    fn init_from_connection(conn: rusqlite::Connection) -> anyhow::Result<Self> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS roots (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                node_id TEXT NOT NULL,
                name TEXT NOT NULL,
                path TEXT NOT NULL,
                UNIQUE (node_id, name)
            )",
            [],
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS files (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                hash_kind TEXT NOT NULL,
                hash TEXT NOT NULL,
                node_id TEXT NOT NULL,
                root TEXT NOT NULL,
                path TEXT NOT NULL,
                local_path TEXT NOT NULL,
                UNIQUE (node_id, root, path)
            )",
            [],
        )?;

        Ok(Self { conn })
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
        let node_id = node_id_to_string(&node_id);
        let mut stmt = self
            .conn
            .prepare("SELECT id, node_id, name, path FROM roots WHERE node_id = ?")
            .expect("should prepare statement");

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
        let node_id = node_id_to_string(&node_id);
        let mut stmt = self
            .conn
            .prepare("SELECT COUNT(*) FROM files WHERE node_id = ? AND root = ?")
            .expect("should prepare statement");

        let count: u64 = stmt
            .query_row([&node_id, root], |row| row.get(0))
            .context("failed to count files")?;

        Ok(count)
    }

    pub fn insert_files<'a>(
        &mut self,
        iter: impl Iterator<Item = InsertFile<'a>>,
    ) -> anyhow::Result<()> {
        let tx = self
            .conn
            .transaction()
            .context("failed to begin transaction")?;

        {
            let mut stmt = tx.prepare("INSERT INTO files (hash_kind, hash, node_id, root, path, local_path) VALUES (?, ?, ?, ?, ?, ?)")?;
            for file in iter {
                stmt.execute((
                    file.hash_kind,
                    file.hash,
                    node_id_to_string(&file.node_id),
                    file.root,
                    file.path,
                    file.local_path,
                ))?;
            }
        }

        tx.commit().context("failed to commit transaction")?;

        Ok(())
    }

    // pub fn get_local_files(&self) -> anyhow::Result<Vec<LocalFile>> {
    //     let mut stmt = self
    //         .conn
    //         .prepare("SELECT id, hash_kind, hash, root, path FROM local_files")
    //         .expect("should prepare statement");

    //     stmt.query_map([], |row| {
    //         Ok(LocalFile {
    //             id: row.get(0)?,
    //             hash_kind: row.get(1)?,
    //             hash: row.get(2)?,
    //             root: row.get(3)?,
    //         })
    //     })
    //     .expect("should bind parameters")
    //     .map(|r| r.map_err(|e| anyhow::anyhow!("failed to map local files: {}", e)))
    //     .collect()
    // }
}

fn node_id_to_string(node_id: &NodeId) -> String {
    hex::encode(node_id)
}
