use crate::node::Node;
use anyhow::Context;
use iroh::NodeId;

pub struct Root {
    pub id: u64,
    pub node_id: NodeId,
    pub name: String,
    pub path: String,
}

#[derive(Debug)]
pub struct Database {
    conn: rusqlite::Connection,
}

impl Database {
    pub fn open(path: &str) -> anyhow::Result<Self> {
        // let conn = rusqlite::Connection::open(path)?;
        let conn = rusqlite::Connection::open_in_memory()?;

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
}

fn node_id_to_string(node_id: &NodeId) -> String {
    hex::encode(node_id)
}
