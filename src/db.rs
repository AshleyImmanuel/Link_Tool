use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::path::Path;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS symbols (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    file TEXT NOT NULL,
    line INTEGER NOT NULL,
    col INTEGER NOT NULL,
    byte_start INTEGER NOT NULL,
    byte_end INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS edges (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    from_id INTEGER NOT NULL,
    to_id INTEGER NOT NULL,
    relation TEXT NOT NULL,
    FOREIGN KEY (from_id) REFERENCES symbols(id),
    FOREIGN KEY (to_id) REFERENCES symbols(id)
);

CREATE TABLE IF NOT EXISTS files (
    path TEXT PRIMARY KEY,
    hash TEXT NOT NULL,
    lang TEXT NOT NULL,
    last_modified INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_symbols_name ON symbols(name);
CREATE INDEX IF NOT EXISTS idx_symbols_file ON symbols(file);
CREATE INDEX IF NOT EXISTS idx_edges_from ON edges(from_id);
CREATE INDEX IF NOT EXISTS idx_edges_to ON edges(to_id);
";

pub struct Db {
    conn: Connection,
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub id: i64,
    pub name: String,
    pub kind: String,
    pub file: String,
    pub line: u32,
    pub col: u32,
    pub byte_start: u32,
    pub byte_end: u32,
}

#[derive(Debug, Clone)]
pub struct Edge {
    pub id: i64,
    pub from_id: i64,
    pub to_id: i64,
    pub relation: String,
}

impl Db {
    pub fn open(link_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(link_dir)
            .with_context(|| format!("failed to create {}", link_dir.display()))?;
        let db_path = link_dir.join("index.db");
        let conn = Connection::open(&db_path)
            .with_context(|| format!("failed to open {}", db_path.display()))?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
            .context("failed to set pragmas")?;
        conn.execute_batch(SCHEMA).context("failed to create schema")?;
        Ok(Self { conn })
    }

    pub fn insert_symbol(
        &self,
        name: &str,
        kind: &str,
        file: &str,
        line: u32,
        col: u32,
        byte_start: u32,
        byte_end: u32,
    ) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO symbols (name, kind, file, line, col, byte_start, byte_end) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![name, kind, file, line, col, byte_start, byte_end],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn insert_edge(&self, from_id: i64, to_id: i64, relation: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO edges (from_id, to_id, relation) VALUES (?1, ?2, ?3)",
            params![from_id, to_id, relation],
        )?;
        Ok(())
    }

    pub fn upsert_file(&self, path: &str, hash: &str, lang: &str, last_modified: i64) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO files (path, hash, lang, last_modified) VALUES (?1, ?2, ?3, ?4)",
            params![path, hash, lang, last_modified],
        )?;
        Ok(())
    }

    pub fn set_meta(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn get_meta(&self, key: &str) -> Result<Option<String>> {
        let mut stmt = self.conn.prepare("SELECT value FROM meta WHERE key = ?1")?;
        let mut rows = stmt.query(params![key])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    pub fn delete_symbols_for_file(&self, file: &str) -> Result<()> {
        // Delete edges referencing symbols from this file
        self.conn.execute(
            "DELETE FROM edges WHERE from_id IN (SELECT id FROM symbols WHERE file = ?1) OR to_id IN (SELECT id FROM symbols WHERE file = ?1)",
            params![file],
        )?;
        self.conn.execute("DELETE FROM symbols WHERE file = ?1", params![file])?;
        Ok(())
    }

    pub fn delete_file(&self, path: &str) -> Result<()> {
        self.delete_symbols_for_file(path)?;
        self.conn.execute("DELETE FROM files WHERE path = ?1", params![path])?;
        Ok(())
    }

    pub fn get_file_hash(&self, path: &str) -> Result<Option<String>> {
        let mut stmt = self.conn.prepare("SELECT hash FROM files WHERE path = ?1")?;
        let mut rows = stmt.query(params![path])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    pub fn all_file_paths(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare("SELECT path FROM files")?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        let mut paths = Vec::new();
        for r in rows {
            paths.push(r?);
        }
        Ok(paths)
    }

    pub fn find_symbols_by_name(&self, name: &str) -> Result<Vec<Symbol>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, kind, file, line, col, byte_start, byte_end FROM symbols WHERE name = ?1",
        )?;
        let rows = stmt.query_map(params![name], map_symbol)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn fuzzy_search(&self, query: &str) -> Result<Vec<Symbol>> {
        let pattern = format!("%{}%", query);
        let mut stmt = self.conn.prepare(
            "SELECT id, name, kind, file, line, col, byte_start, byte_end FROM symbols WHERE name LIKE ?1 ORDER BY name LIMIT 200",
        )?;
        let rows = stmt.query_map(params![pattern], map_symbol)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn list_all_symbols(&self) -> Result<Vec<Symbol>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, kind, file, line, col, byte_start, byte_end FROM symbols ORDER BY file, line",
        )?;
        let rows = stmt.query_map([], map_symbol)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn edges_for_symbol(&self, symbol_id: i64) -> Result<Vec<(Edge, Symbol)>> {
        // Get edges where this symbol is source or target, plus the other symbol
        let mut results = Vec::new();

        // Outgoing edges (this symbol calls/uses others)
        let mut stmt = self.conn.prepare(
            "SELECT e.id, e.from_id, e.to_id, e.relation, s.id, s.name, s.kind, s.file, s.line, s.col, s.byte_start, s.byte_end \
             FROM edges e JOIN symbols s ON e.to_id = s.id WHERE e.from_id = ?1",
        )?;
        let rows = stmt.query_map(params![symbol_id], |row| {
            Ok((
                Edge { id: row.get(0)?, from_id: row.get(1)?, to_id: row.get(2)?, relation: row.get(3)? },
                Symbol { id: row.get(4)?, name: row.get(5)?, kind: row.get(6)?, file: row.get(7)?, line: row.get(8)?, col: row.get(9)?, byte_start: row.get(10)?, byte_end: row.get(11)? },
            ))
        })?;
        for r in rows {
            results.push(r?);
        }

        // Incoming edges (others call/use this symbol)
        let mut stmt = self.conn.prepare(
            "SELECT e.id, e.from_id, e.to_id, e.relation, s.id, s.name, s.kind, s.file, s.line, s.col, s.byte_start, s.byte_end \
             FROM edges e JOIN symbols s ON e.from_id = s.id WHERE e.to_id = ?1",
        )?;
        let rows = stmt.query_map(params![symbol_id], |row| {
            Ok((
                Edge { id: row.get(0)?, from_id: row.get(1)?, to_id: row.get(2)?, relation: row.get(3)? },
                Symbol { id: row.get(4)?, name: row.get(5)?, kind: row.get(6)?, file: row.get(7)?, line: row.get(8)?, col: row.get(9)?, byte_start: row.get(10)?, byte_end: row.get(11)? },
            ))
        })?;
        for r in rows {
            results.push(r?);
        }

        Ok(results)
    }

    pub fn symbol_count(&self) -> Result<i64> {
        Ok(self.conn.query_row("SELECT COUNT(*) FROM symbols", [], |r| r.get(0))?)
    }

    pub fn edge_count(&self) -> Result<i64> {
        Ok(self.conn.query_row("SELECT COUNT(*) FROM edges", [], |r| r.get(0))?)
    }

    pub fn file_count(&self) -> Result<i64> {
        Ok(self.conn.query_row("SELECT COUNT(*) FROM files", [], |r| r.get(0))?)
    }

    pub fn vacuum(&self) -> Result<()> {
        self.conn.execute_batch("VACUUM")?;
        Ok(())
    }

    pub fn all_symbols(&self) -> Result<Vec<Symbol>> {
        self.list_all_symbols()
    }

    pub fn all_edges(&self) -> Result<Vec<Edge>> {
        let mut stmt = self.conn.prepare("SELECT id, from_id, to_id, relation FROM edges")?;
        let rows = stmt.query_map([], |row| {
            Ok(Edge {
                id: row.get(0)?,
                from_id: row.get(1)?,
                to_id: row.get(2)?,
                relation: row.get(3)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }
}

fn map_symbol(row: &rusqlite::Row) -> rusqlite::Result<Symbol> {
    Ok(Symbol {
        id: row.get(0)?,
        name: row.get(1)?,
        kind: row.get(2)?,
        file: row.get(3)?,
        line: row.get(4)?,
        col: row.get(5)?,
        byte_start: row.get(6)?,
        byte_end: row.get(7)?,
    })
}
