pub const DEFINITION_KIND_FILTER: &str =
    "'function','class','method','variable','struct','enum','type','interface','module','route'";
pub const INDEX_FORMAT_META_KEY: &str = "index_format_version";
pub const INDEX_FORMAT_VERSION: &str = "5";

pub const SCHEMA: &str = "
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
    reason TEXT NOT NULL,
    origin_file TEXT NOT NULL,
    origin_line INTEGER NOT NULL,
    confidence REAL NOT NULL,
    FOREIGN KEY (from_id) REFERENCES symbols(id),
    FOREIGN KEY (to_id) REFERENCES symbols(id)
);

CREATE TABLE IF NOT EXISTS import_refs (
    file TEXT NOT NULL,
    imported_name TEXT NOT NULL,
    source_module TEXT NOT NULL,
    line INTEGER NOT NULL,
    PRIMARY KEY (file, imported_name, source_module, line)
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

CREATE TABLE IF NOT EXISTS command_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ts INTEGER NOT NULL,
    session_key TEXT NOT NULL,
    cwd TEXT NOT NULL,
    command TEXT NOT NULL,
    success INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS route_refs (
    route_id INTEGER NOT NULL,
    handler_name TEXT NOT NULL,
    origin_file TEXT NOT NULL,
    origin_line INTEGER NOT NULL,
    FOREIGN KEY (route_id) REFERENCES symbols(id)
);

CREATE INDEX IF NOT EXISTS idx_symbols_name ON symbols(name);
CREATE INDEX IF NOT EXISTS idx_symbols_kind ON symbols(kind);
CREATE INDEX IF NOT EXISTS idx_symbols_name_kind ON symbols(name, kind);
CREATE INDEX IF NOT EXISTS idx_symbols_file ON symbols(file);
CREATE INDEX IF NOT EXISTS idx_edges_from ON edges(from_id);
CREATE INDEX IF NOT EXISTS idx_edges_to ON edges(to_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_edges_unique ON edges(from_id, to_id, relation);
CREATE INDEX IF NOT EXISTS idx_import_refs_file ON import_refs(file);
CREATE INDEX IF NOT EXISTS idx_import_refs_module ON import_refs(source_module);
CREATE INDEX IF NOT EXISTS idx_command_history_ts ON command_history(ts);
CREATE INDEX IF NOT EXISTS idx_command_history_session ON command_history(session_key);
CREATE INDEX IF NOT EXISTS idx_route_refs_route ON route_refs(route_id);
CREATE INDEX IF NOT EXISTS idx_route_refs_handler ON route_refs(handler_name);
";
