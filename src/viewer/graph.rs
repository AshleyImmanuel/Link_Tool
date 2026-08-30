use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;

use anyhow::Result;

use crate::db::{Db, Edge, Symbol};
use crate::intel;

const MAX_IMPACT_DEPTH: usize = 4;
const MAX_GRAPH_NODES: usize = 64;

/// Graph data structure for JSON output and HTML rendering.
#[derive(Debug)]
pub struct GraphData {
    pub center_symbol: Symbol,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Debug)]
pub struct GraphNode {
    pub id: i64,
    pub label: String,
    pub kind: String,
    pub title: String,
    pub file: String,
    pub line: u32,
    pub col: u32,
    pub is_center: bool,
    pub is_changed: bool,
    pub impact_depth: usize,
}

#[derive(Debug)]
pub struct GraphEdge {
    pub from: i64,
    pub to: i64,
    pub label: String,
    pub title: String,
    pub changed: bool,
}

/// Build an impact-oriented graph for a symbol.
pub fn build_graph(db: &Db, symbol: &Symbol, project_root: &Path) -> Result<GraphData> {
    let changed_files = intel::changed_files(project_root)?.unwrap_or_default();
    let all_symbols = db.list_all_symbols()?;
    let all_edges = db.all_edges()?;

    let mut symbol_by_id = HashMap::new();
    for candidate in all_symbols {
        symbol_by_id.insert(candidate.id, candidate);
    }

    let mut incoming: HashMap<i64, Vec<Edge>> = HashMap::new();
    let mut outgoing: HashMap<i64, Vec<Edge>> = HashMap::new();
    for edge in all_edges {
        incoming.entry(edge.to_id).or_default().push(edge.clone());
        outgoing.entry(edge.from_id).or_default().push(edge);
    }

    let mut included_ids = HashSet::new();
    let mut impact_depths = HashMap::new();
    let mut included_edges = Vec::new();
    let mut seen_edges = HashSet::new();
    let mut queue = VecDeque::new();

    included_ids.insert(symbol.id);
    impact_depths.insert(symbol.id, 0usize);
    queue.push_back((symbol.id, 0usize));

    while let Some((current, depth)) = queue.pop_front() {
        if depth >= MAX_IMPACT_DEPTH || included_ids.len() >= MAX_GRAPH_NODES {
            continue;
        }

        for edge in incoming.get(&current).into_iter().flatten() {
            if seen_edges.insert(edge_key(edge)) {
                included_edges.push(edge.clone());
            }

            if included_ids.len() >= MAX_GRAPH_NODES {
                continue;
            }

            if included_ids.insert(edge.from_id) {
                impact_depths.insert(edge.from_id, depth + 1);
                queue.push_back((edge.from_id, depth + 1));
            }
        }
    }

    for edge in outgoing.get(&symbol.id).into_iter().flatten() {
        if seen_edges.insert(edge_key(edge)) {
            included_edges.push(edge.clone());
        }
        if included_ids.len() >= MAX_GRAPH_NODES {
            continue;
        }
        if included_ids.insert(edge.to_id) {
            impact_depths.insert(edge.to_id, 0);
        }
    }

    let mut nodes = Vec::new();
    for id in &included_ids {
        let Some(other) = symbol_by_id.get(id) else {
            continue;
        };
        let semantic_kind = intel::semantic_kind(other).to_string();
        let changed = changed_files.contains(&other.file);
        let impact_depth = *impact_depths.get(id).unwrap_or(&0);

        nodes.push(GraphNode {
            id: other.id,
            label: intel::semantic_label(other),
            kind: semantic_kind.clone(),
            title: build_node_title(other, &semantic_kind, changed, impact_depth),
            file: other.file.clone(),
            line: other.line,
            col: other.col,
            is_center: other.id == symbol.id,
            is_changed: changed,
            impact_depth,
        });
    }

    nodes.sort_by(|left, right| {
        left.impact_depth
            .cmp(&right.impact_depth)
            .then_with(|| left.file.cmp(&right.file))
            .then_with(|| left.line.cmp(&right.line))
            .then_with(|| left.label.cmp(&right.label))
    });

    let mut edges = Vec::new();
    for edge in included_edges {
        let changed = changed_files.contains(&edge.origin_file)
            || symbol_by_id
                .get(&edge.from_id)
                .map(|symbol| changed_files.contains(&symbol.file))
                .unwrap_or(false)
            || symbol_by_id
                .get(&edge.to_id)
                .map(|symbol| changed_files.contains(&symbol.file))
                .unwrap_or(false);

        edges.push(GraphEdge {
            from: edge.from_id,
            to: edge.to_id,
            label: edge.relation.clone(),
            title: build_edge_title(&edge),
            changed,
        });
    }

    Ok(GraphData {
        center_symbol: symbol.clone(),
        nodes,
        edges,
    })
}

fn build_node_title(
    symbol: &Symbol,
    semantic_kind: &str,
    changed: bool,
    impact_depth: usize,
) -> String {
    let mut title = format!(
        "{}\nkind: {}\nfile: {}\nline: {}",
        symbol.name, semantic_kind, symbol.file, symbol.line
    );
    if changed {
        title.push_str("\nchanged in local git working tree (vs HEAD)");
    }
    if impact_depth > 0 {
        title.push_str(&format!("\nimpact depth: {}", impact_depth));
    }
    if let Some(route) = intel::route_path_for_file(&symbol.file) {
        title.push_str(&format!("\nroute: {}", route));
    }
    title
}

fn build_edge_title(edge: &Edge) -> String {
    format!(
        "{}\nwhy: {}\norigin: {}:{}\nconfidence: {}%",
        edge.relation,
        edge.reason,
        edge.origin_file,
        edge.origin_line,
        (edge.confidence * 100.0).round()
    )
}

fn edge_key(edge: &Edge) -> (i64, i64, String) {
    (edge.from_id, edge.to_id, edge.relation.clone())
}
