use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::time::Duration;

use crate::error::user_error;

const DEFINITION_KIND_FILTER: &str =
    "'function','class','method','variable','struct','enum','type','interface','module'";
const INDEX_FORMAT_META_KEY: &str = "index_format_version";
const INDEX_FORMAT_VERSION: &str = "2";

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
CREATE INDEX IF NOT EXISTS idx_symbols_kind ON symbols(kind);
CREATE INDEX IF NOT EXISTS idx_symbols_name_kind ON symbols(name, kind);
CREATE INDEX IF NOT EXISTS idx_symbols_file ON symbols(file);
CREATE INDEX IF NOT EXISTS idx_edges_from ON edges(from_id);
CREATE INDEX IF NOT EXISTS idx_edges_to ON edges(to_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_edges_unique ON edges(from_id, to_id, relation);
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
}

#[derive(Debug, Clone)]
pub struct Edge {
    pub from_id: i64,
    pub to_id: i64,
    pub relation: String,
}

impl Db {
    pub fn open(link_dir: &Path) -> Result<Self> {
        ensure_safe_link_dir(link_dir)?;
        let db_path = link_dir.join("index.db");
        let conn = Connection::open(&db_path)
            .with_context(|| format!("failed to open {}", db_path.display()))?;
        conn.busy_timeout(Duration::from_secs(5))
            .context("failed to configure sqlite busy timeout")?;
        conn.execute_batch(
            "PRAGMA foreign_keys=ON; \
             PRAGMA journal_mode=WAL; \
             PRAGMA synchronous=NORMAL; \
             PRAGMA temp_store=MEMORY;",
        )
        .context("failed to set pragmas")?;
        conn.execute_batch(SCHEMA)
            .context("failed to create schema")?;
        Ok(Self { conn })
    }

    pub fn open_index(link_dir: &Path) -> Result<Self> {
        let db = Self::open(link_dir)?;
        db.require_current_index()?;
        Ok(db)
    }

    pub fn with_transaction<T, F>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&Self) -> Result<T>,
    {
        self.begin_transaction()?;

        match f(self) {
            Ok(value) => {
                self.commit_transaction()?;
                Ok(value)
            }
            Err(err) => {
                let _ = self.rollback_transaction();
                Err(err)
            }
        }
    }

    pub fn begin_transaction(&self) -> Result<()> {
        self.conn
            .execute_batch("BEGIN IMMEDIATE TRANSACTION")
            .context("failed to begin sqlite transaction")?;
        Ok(())
    }

    pub fn commit_transaction(&self) -> Result<()> {
        self.conn
            .execute_batch("COMMIT")
            .context("failed to commit sqlite transaction")?;
        Ok(())
    }

    pub fn rollback_transaction(&self) -> Result<()> {
        self.conn
            .execute_batch("ROLLBACK")
            .context("failed to rollback sqlite transaction")?;
        Ok(())
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
            "INSERT OR IGNORE INTO edges (from_id, to_id, relation) VALUES (?1, ?2, ?3)",
            params![from_id, to_id, relation],
        )?;
        Ok(())
    }

    pub fn upsert_file(
        &self,
        path: &str,
        hash: &str,
        lang: &str,
        last_modified: i64,
    ) -> Result<()> {
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
        self.conn
            .query_row(
                "SELECT value FROM meta WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn write_index_metadata(&self) -> Result<()> {
        self.set_meta(INDEX_FORMAT_META_KEY, INDEX_FORMAT_VERSION)
    }

    pub fn delete_symbols_for_file(&self, file: &str) -> Result<()> {
        // Delete edges referencing symbols from this file
        self.conn.execute(
            "DELETE FROM edges WHERE from_id IN (SELECT id FROM symbols WHERE file = ?1) OR to_id IN (SELECT id FROM symbols WHERE file = ?1)",
            params![file],
        )?;
        self.conn
            .execute("DELETE FROM symbols WHERE file = ?1", params![file])?;
        Ok(())
    }

    pub fn delete_file(&self, path: &str) -> Result<()> {
        self.delete_symbols_for_file(path)?;
        self.conn
            .execute("DELETE FROM files WHERE path = ?1", params![path])?;
        Ok(())
    }

    pub fn get_file_hash(&self, path: &str) -> Result<Option<String>> {
        self.conn
            .query_row(
                "SELECT hash FROM files WHERE path = ?1",
                params![path],
                |row| row.get(0),
            )
            .optional()
            .map_err(Into::into)
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
            "SELECT id, name, kind, file, line, col \
             FROM symbols WHERE name = ?1 ORDER BY file, line, col",
        )?;
        let rows = stmt.query_map(params![name], map_symbol)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn fuzzy_search(&self, query: &str) -> Result<Vec<Symbol>> {
        let pattern = format!("%{}%", escape_like_pattern(query));
        let mut stmt = self.conn.prepare(
            "SELECT id, name, kind, file, line, col \
             FROM symbols WHERE name LIKE ?1 ESCAPE '\\' ORDER BY name LIMIT 200",
        )?;
        let rows = stmt.query_map(params![pattern], map_symbol)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn list_all_symbols(&self) -> Result<Vec<Symbol>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, kind, file, line, col \
             FROM symbols ORDER BY file, line, col",
        )?;
        let rows = stmt.query_map([], map_symbol)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn definition_symbols(&self) -> Result<Vec<Symbol>> {
        let sql = format!(
            "SELECT id, name, kind, file, line, col \
             FROM symbols WHERE kind IN ({}) ORDER BY name, file, line, col",
            DEFINITION_KIND_FILTER
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map([], map_symbol)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn symbols_by_kind(&self, kind: &str) -> Result<Vec<Symbol>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, kind, file, line, col \
             FROM symbols WHERE kind = ?1 ORDER BY file, line, col",
        )?;
        let rows = stmt.query_map(params![kind], map_symbol)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn edges_for_symbol(&self, symbol_id: i64) -> Result<Vec<(Edge, Symbol)>> {
        // Get edges where this symbol is source or target, plus the other symbol
        let mut results = Vec::new();

        // Outgoing edges (this symbol calls/uses others)
        let mut stmt = self.conn.prepare(
            "SELECT e.from_id, e.to_id, e.relation, s.id, s.name, s.kind, s.file, s.line, s.col \
             FROM edges e JOIN symbols s ON e.to_id = s.id WHERE e.from_id = ?1",
        )?;
        let rows = stmt.query_map(params![symbol_id], |row| {
            Ok((
                Edge {
                    from_id: row.get(0)?,
                    to_id: row.get(1)?,
                    relation: row.get(2)?,
                },
                Symbol {
                    id: row.get(3)?,
                    name: row.get(4)?,
                    kind: row.get(5)?,
                    file: row.get(6)?,
                    line: row.get(7)?,
                    col: row.get(8)?,
                },
            ))
        })?;
        for r in rows {
            results.push(r?);
        }

        // Incoming edges (others call/use this symbol)
        let mut stmt = self.conn.prepare(
            "SELECT e.from_id, e.to_id, e.relation, s.id, s.name, s.kind, s.file, s.line, s.col \
             FROM edges e JOIN symbols s ON e.from_id = s.id WHERE e.to_id = ?1",
        )?;
        let rows = stmt.query_map(params![symbol_id], |row| {
            Ok((
                Edge {
                    from_id: row.get(0)?,
                    to_id: row.get(1)?,
                    relation: row.get(2)?,
                },
                Symbol {
                    id: row.get(3)?,
                    name: row.get(4)?,
                    kind: row.get(5)?,
                    file: row.get(6)?,
                    line: row.get(7)?,
                    col: row.get(8)?,
                },
            ))
        })?;
        for r in rows {
            results.push(r?);
        }

        Ok(results)
    }

    pub fn symbol_count(&self) -> Result<i64> {
        Ok(self
            .conn
            .query_row("SELECT COUNT(*) FROM symbols", [], |r| r.get(0))?)
    }

    pub fn edge_count(&self) -> Result<i64> {
        Ok(self
            .conn
            .query_row("SELECT COUNT(*) FROM edges", [], |r| r.get(0))?)
    }

    pub fn file_count(&self) -> Result<i64> {
        Ok(self
            .conn
            .query_row("SELECT COUNT(*) FROM files", [], |r| r.get(0))?)
    }

    pub fn clear_edges(&self) -> Result<()> {
        self.conn.execute("DELETE FROM edges", [])?;
        Ok(())
    }

    pub fn reset_index(&self) -> Result<()> {
        self.conn.execute_batch(
            "DELETE FROM edges; \
             DELETE FROM symbols; \
             DELETE FROM files; \
             DELETE FROM meta;",
        )?;
        Ok(())
    }

    fn require_current_index(&self) -> Result<()> {
        match self.get_meta(INDEX_FORMAT_META_KEY)? {
            Some(version) if version == INDEX_FORMAT_VERSION => Ok(()),
            Some(version) => Err(user_error(format!(
                "index format {} is not supported by this build. Run 'link init' to rebuild the index.",
                version
            ))),
            None => Err(user_error(
                "index format is missing or out of date. Run 'link init' to rebuild the index.",
            )),
        }
    }
}

fn ensure_safe_link_dir(link_dir: &Path) -> Result<()> {
    if link_dir.file_name().and_then(|name| name.to_str()) != Some(".link") {
        return Err(user_error("refusing to write outside the .link directory"));
    }

    match std::fs::symlink_metadata(link_dir) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                return Err(user_error("refusing to use a symlinked .link directory"));
            }
            if !metadata.is_dir() {
                return Err(user_error(".link exists but is not a directory"));
            }
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            std::fs::create_dir_all(link_dir)
                .with_context(|| format!("failed to create {}", link_dir.display()))?;
        }
        Err(err) => {
            return Err(err).with_context(|| format!("failed to inspect {}", link_dir.display()));
        }
    }

    Ok(())
}

fn escape_like_pattern(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '%' => escaped.push_str("\\%"),
            '_' => escaped.push_str("\\_"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn map_symbol(row: &rusqlite::Row) -> rusqlite::Result<Symbol> {
    Ok(Symbol {
        id: row.get(0)?,
        name: row.get(1)?,
        kind: row.get(2)?,
        file: row.get(3)?,
        line: row.get(4)?,
        col: row.get(5)?,
    })
}
