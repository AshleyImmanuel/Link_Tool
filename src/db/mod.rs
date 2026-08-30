mod models;
mod schema;

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::time::Duration;

use crate::error::user_error;
use schema::{DEFINITION_KIND_FILTER, INDEX_FORMAT_META_KEY, INDEX_FORMAT_VERSION, SCHEMA};

pub use models::{CommandHistoryEntry, Edge, ImportRef, RouteRef, Symbol};

pub struct Db {
    conn: Connection,
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

    #[allow(clippy::too_many_arguments)]
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

    #[allow(clippy::too_many_arguments)]
    pub fn insert_edge(
        &self,
        from_id: i64,
        to_id: i64,
        relation: &str,
        reason: &str,
        origin_file: &str,
        origin_line: u32,
        confidence: f32,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO edges (from_id, to_id, relation, reason, origin_file, origin_line, confidence) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![from_id, to_id, relation, reason, origin_file, origin_line, confidence],
        )?;
        Ok(())
    }

    pub fn insert_import_ref(
        &self,
        file: &str,
        imported_name: &str,
        source_module: &str,
        line: u32,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO import_refs (file, imported_name, source_module, line) VALUES (?1, ?2, ?3, ?4)",
            params![file, imported_name, source_module, line],
        )?;
        Ok(())
    }

    pub fn insert_route_ref(
        &self,
        route_id: i64,
        handler_name: &str,
        origin_file: &str,
        origin_line: u32,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO route_refs (route_id, handler_name, origin_file, origin_line) VALUES (?1, ?2, ?3, ?4)",
            params![route_id, handler_name, origin_file, origin_line],
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

    pub fn insert_command_history(
        &self,
        ts: u64,
        session_key: &str,
        cwd: &str,
        command: &str,
        success: bool,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO command_history (ts, session_key, cwd, command, success) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![ts, session_key, cwd, command, success as i64],
        )?;
        Ok(())
    }

    pub fn command_history(
        &self,
        session_key: Option<&str>,
        limit: usize,
    ) -> Result<Vec<CommandHistoryEntry>> {
        let mut results = Vec::new();

        if let Some(session_key) = session_key {
            let mut stmt = self.conn.prepare(
                "SELECT ts, session_key, command, success
                 FROM command_history
                 WHERE session_key = ?1
                 ORDER BY id DESC
                 LIMIT ?2",
            )?;
            let rows = stmt.query_map(params![session_key, limit as i64], |row| {
                Ok(CommandHistoryEntry {
                    ts: row.get(0)?,
                    session_key: row.get(1)?,
                    command: row.get(2)?,
                    success: row.get::<_, i64>(3)? != 0,
                })
            })?;
            for row in rows {
                results.push(row?);
            }
        } else {
            let mut stmt = self.conn.prepare(
                "SELECT ts, session_key, command, success
                 FROM command_history
                 ORDER BY id DESC
                 LIMIT ?1",
            )?;
            let rows = stmt.query_map(params![limit as i64], |row| {
                Ok(CommandHistoryEntry {
                    ts: row.get(0)?,
                    session_key: row.get(1)?,
                    command: row.get(2)?,
                    success: row.get::<_, i64>(3)? != 0,
                })
            })?;
            for row in rows {
                results.push(row?);
            }
        }

        Ok(results)
    }

    pub fn delete_symbols_for_file(&self, file: &str) -> Result<()> {
        self.delete_import_refs_for_file(file)?;
        self.delete_route_refs_for_file(file)?;
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

    pub fn all_edges(&self) -> Result<Vec<Edge>> {
        let mut stmt = self.conn.prepare(
            "SELECT from_id, to_id, relation, reason, origin_file, origin_line, confidence \
             FROM edges ORDER BY from_id, to_id, relation",
        )?;
        let rows = stmt.query_map([], map_edge)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
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

    pub fn all_import_refs(&self) -> Result<Vec<ImportRef>> {
        let mut stmt = self.conn.prepare(
            "SELECT file, imported_name, source_module, line \
             FROM import_refs ORDER BY file, line, imported_name",
        )?;
        let rows = stmt.query_map([], map_import_ref)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn import_refs_for_file(&self, file: &str) -> Result<Vec<ImportRef>> {
        let mut stmt = self.conn.prepare(
            "SELECT file, imported_name, source_module, line \
             FROM import_refs WHERE file = ?1 ORDER BY line, imported_name",
        )?;
        let rows = stmt.query_map(params![file], map_import_ref)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn all_route_refs(&self) -> Result<Vec<RouteRef>> {
        let mut stmt = self.conn.prepare(
            "SELECT route_id, handler_name, origin_file, origin_line \
             FROM route_refs ORDER BY route_id, origin_line",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(RouteRef {
                route_id: row.get(0)?,
                handler_name: row.get(1)?,
                origin_file: row.get(2)?,
                origin_line: row.get::<_, i64>(3)? as u32,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn edges_for_symbol(&self, symbol_id: i64) -> Result<Vec<(Edge, Symbol)>> {
        let mut results = Vec::new();

        let mut stmt = self.conn.prepare(
            "SELECT e.from_id, e.to_id, e.relation, e.reason, e.origin_file, e.origin_line, e.confidence, s.id, s.name, s.kind, s.file, s.line, s.col \
             FROM edges e JOIN symbols s ON e.to_id = s.id WHERE e.from_id = ?1",
        )?;
        let rows = stmt.query_map(params![symbol_id], |row| {
            Ok((
                Edge {
                    from_id: row.get(0)?,
                    to_id: row.get(1)?,
                    relation: row.get(2)?,
                    reason: row.get(3)?,
                    origin_file: row.get(4)?,
                    origin_line: row.get(5)?,
                    confidence: row.get(6)?,
                },
                Symbol {
                    id: row.get(7)?,
                    name: row.get(8)?,
                    kind: row.get(9)?,
                    file: row.get(10)?,
                    line: row.get(11)?,
                    col: row.get(12)?,
                },
            ))
        })?;
        for r in rows {
            results.push(r?);
        }

        let mut stmt = self.conn.prepare(
            "SELECT e.from_id, e.to_id, e.relation, e.reason, e.origin_file, e.origin_line, e.confidence, s.id, s.name, s.kind, s.file, s.line, s.col \
             FROM edges e JOIN symbols s ON e.from_id = s.id WHERE e.to_id = ?1",
        )?;
        let rows = stmt.query_map(params![symbol_id], |row| {
            Ok((
                Edge {
                    from_id: row.get(0)?,
                    to_id: row.get(1)?,
                    relation: row.get(2)?,
                    reason: row.get(3)?,
                    origin_file: row.get(4)?,
                    origin_line: row.get(5)?,
                    confidence: row.get(6)?,
                },
                Symbol {
                    id: row.get(7)?,
                    name: row.get(8)?,
                    kind: row.get(9)?,
                    file: row.get(10)?,
                    line: row.get(11)?,
                    col: row.get(12)?,
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
            "DROP TABLE IF EXISTS edges; \
             DROP TABLE IF EXISTS symbols; \
             DROP TABLE IF EXISTS files; \
             DROP TABLE IF EXISTS meta; \
             DROP TABLE IF EXISTS import_refs; \
             DROP TABLE IF EXISTS route_refs;",
        )?;
        self.conn.execute_batch(SCHEMA)?;
        Ok(())
    }

    fn require_current_index(&self) -> Result<()> {
        match self.get_meta(INDEX_FORMAT_META_KEY)? {
            Some(version) if version == INDEX_FORMAT_VERSION => Ok(()),
            Some(version) => Err(user_error(format!(
                "index format {} is not supported by this build. Run 'linkmap init' to rebuild the index.",
                version
            ))),
            None => Err(user_error(
                "index format is missing or out of date. Run 'linkmap init' to rebuild the index.",
            )),
        }
    }

    fn delete_import_refs_for_file(&self, file: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM import_refs WHERE file = ?1", params![file])?;
        Ok(())
    }

    fn delete_route_refs_for_file(&self, file: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM route_refs WHERE origin_file = ?1 OR route_id IN (SELECT id FROM symbols WHERE file = ?1)",
            params![file],
        )?;
        Ok(())
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

fn map_edge(row: &rusqlite::Row) -> rusqlite::Result<Edge> {
    Ok(Edge {
        from_id: row.get(0)?,
        to_id: row.get(1)?,
        relation: row.get(2)?,
        reason: row.get(3)?,
        origin_file: row.get(4)?,
        origin_line: row.get(5)?,
        confidence: row.get(6)?,
    })
}

fn map_import_ref(row: &rusqlite::Row) -> rusqlite::Result<ImportRef> {
    Ok(ImportRef {
        file: row.get(0)?,
        imported_name: row.get(1)?,
        source_module: row.get(2)?,
        line: row.get(3)?,
    })
}
