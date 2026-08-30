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
    pub reason: String,
    pub origin_file: String,
    pub origin_line: u32,
    pub confidence: f32,
}

#[derive(Debug, Clone)]
pub struct ImportRef {
    pub file: String,
    pub imported_name: String,
    pub source_module: String,
    pub line: u32,
}

#[derive(Debug, Clone)]
pub struct RouteRef {
    pub route_id: i64,
    pub handler_name: String,
    pub origin_file: String,
    pub origin_line: u32,
}

#[derive(Debug, Clone)]
pub struct CommandHistoryEntry {
    pub ts: u64,
    pub session_key: String,
    pub command: String,
    pub success: bool,
}
