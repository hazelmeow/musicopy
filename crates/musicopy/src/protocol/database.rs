pub struct LocalRoot {
    pub id: u64,
    pub path: String,
}

pub struct LocalFile {
    pub id: u64,
    pub hash_kind: String,
    pub hash: String,
    pub root: String,
}

#[derive(Debug)]
pub struct Database {
    conn: rusqlite::Connection,
}

impl Database {
    pub fn open(path: &str) -> anyhow::Result<Self> {
        let conn = rusqlite::Connection::open(path)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS local_roots (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                path TEXT NOT NULL
            )",
            [],
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS local_files (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                hash_kind TEXT NOT NULL,
                hash TEXT NOT NULL,
                root TEXT NOT NULL,
                path TEXT NOT NULL,
                UNIQUE (root, path)
            )",
            [],
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS remote_files (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                hash_kind TEXT NOT NULL,
                hash TEXT NOT NULL,
                remote_node_id TEXT NOT NULL,
                remote_root TEXT NOT NULL,
                remote_path TEXT NOT NULL,
                UNIQUE (remote_node_id, remote_root, remote_path)
            )",
            [],
        )?;

        Ok(Self { conn })
    }

    pub fn get_local_roots(&self) -> anyhow::Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT path FROM local_roots")
            .expect("should prepare statement");

        stmt.query_map([], |row| row.get(0))
            .expect("should bind parameters")
            .map(|r| r.map_err(|e| anyhow::anyhow!("failed to map local roots: {}", e)))
            .collect()
    }

    pub fn add_local_root(&self, path: &str) -> anyhow::Result<u64> {
        self.conn
            .execute("INSERT INTO local_roots (path) VALUES (?)", [path])?;
        Ok(self.conn.last_insert_rowid() as u64)
    }

    pub fn remove_local_root(&self, path: &str) -> anyhow::Result<()> {
        self.conn
            .execute("DELETE FROM local_roots WHERE path = ?", [path])?;
        Ok(())
    }

    pub fn get_local_files(&self) -> anyhow::Result<Vec<LocalFile>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, hash_kind, hash, root, path FROM local_files")
            .expect("should prepare statement");

        stmt.query_map([], |row| {
            Ok(LocalFile {
                id: row.get(0)?,
                hash_kind: row.get(1)?,
                hash: row.get(2)?,
                root: row.get(3)?,
            })
        })
        .expect("should bind parameters")
        .map(|r| r.map_err(|e| anyhow::anyhow!("failed to map local files: {}", e)))
        .collect()
    }
}
