use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use std::path::{Component, Path};

use crate::db::{Db, Symbol};

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
    pub file: String,
    pub line: u32,
    pub col: u32,
    pub is_center: bool,
}

#[derive(Debug)]
pub struct GraphEdge {
    pub from: i64,
    pub to: i64,
    pub label: String,
}

/// Build the graph data for a symbol (its neighborhood: callers + callees).
pub fn build_graph(db: &Db, symbol: &Symbol) -> Result<GraphData> {
    let edges_and_symbols = db.edges_for_symbol(symbol.id)?;

    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    // Center node
    nodes.push(GraphNode {
        id: symbol.id,
        label: format!(
            "{} ({}:{})",
            symbol.name,
            short_path(&symbol.file),
            symbol.line
        ),
        kind: symbol.kind.clone(),
        file: symbol.file.clone(),
        line: symbol.line,
        col: symbol.col,
        is_center: true,
    });
    seen_ids.insert(symbol.id);

    for (edge, other) in &edges_and_symbols {
        if !seen_ids.contains(&other.id) {
            nodes.push(GraphNode {
                id: other.id,
                label: format!(
                    "{} ({}:{})",
                    other.name,
                    short_path(&other.file),
                    other.line
                ),
                kind: other.kind.clone(),
                file: other.file.clone(),
                line: other.line,
                col: other.col,
                is_center: false,
            });
            seen_ids.insert(other.id);
        }
        edges.push(GraphEdge {
            from: edge.from_id,
            to: edge.to_id,
            label: edge.relation.clone(),
        });
    }

    Ok(GraphData {
        center_symbol: symbol.clone(),
        nodes,
        edges,
    })
}

/// Serialize graph data to JSON string.
pub fn graph_to_json(data: &GraphData) -> String {
    let mut json = String::from("{\n");

    // Center
    json.push_str(&format!(
        "  \"center\": {{ \"id\": {}, \"name\": \"{}\", \"kind\": \"{}\", \"file\": \"{}\", \"line\": {} }},\n",
        data.center_symbol.id,
        escape_json(&data.center_symbol.name),
        escape_json(&data.center_symbol.kind),
        escape_json(&data.center_symbol.file),
        data.center_symbol.line
    ));

    // Nodes
    json.push_str("  \"nodes\": [\n");
    for (i, n) in data.nodes.iter().enumerate() {
        json.push_str(&format!(
            "    {{ \"id\": {}, \"label\": \"{}\", \"kind\": \"{}\", \"file\": \"{}\", \"line\": {}, \"col\": {}, \"center\": {} }}{}",
            n.id,
            escape_json(&n.label),
            escape_json(&n.kind),
            escape_json(&n.file),
            n.line,
            n.col,
            n.is_center,
            if i + 1 < data.nodes.len() { ",\n" } else { "\n" }
        ));
    }
    json.push_str("  ],\n");

    // Edges
    json.push_str("  \"edges\": [\n");
    for (i, e) in data.edges.iter().enumerate() {
        json.push_str(&format!(
            "    {{ \"from\": {}, \"to\": {}, \"label\": \"{}\" }}{}",
            e.from,
            e.to,
            escape_json(&e.label),
            if i + 1 < data.edges.len() {
                ",\n"
            } else {
                "\n"
            }
        ));
    }
    json.push_str("  ]\n");
    json.push('}');
    json
}

/// Generate self-contained HTML with embedded vis-network for the graph.
pub fn generate_html(data: &GraphData, repo_root: &Path) -> String {
    let center_name = escape_html(&data.center_symbol.name);
    let repo_root = repo_root
        .canonicalize()
        .unwrap_or_else(|_| repo_root.to_path_buf());

    // Build nodes JSON array for the template's application/json blocks.
    let mut nodes_json = String::from("[");
    for (i, n) in data.nodes.iter().enumerate() {
        let type_color = kind_color(&n.kind);

        let uri_path = safe_repo_file_path(&repo_root, &n.file).unwrap_or_default();

        let border_width = if n.is_center { 3 } else { 2 };
        let border_color = if n.is_center { "#FFD700" } else { type_color };

        nodes_json.push_str(&format!(
            r##"{{"id":{},"label":"{}","color":{{"background":"#1e1e24","border":"{}","highlight":{{"background":"#2a2a35","border":"{}"}},"hover":{{"background":"#2a2a35","border":"{}"}}}},"borderWidth":{},"font":{{"face":"ui-monospace, SFMono-Regular, Consolas, monospace","color":"#e2e8f0","size":13}},"shape":"box","margin":10,"shadow":{{"enabled":true,"color":"rgba(0,0,0,0.5)","size":5,"x":2,"y":2}},"file":"{}","line":{},"col":{}}}"##,
            n.id,
            escape_js(&n.label),
            border_color,
            border_color,
            border_color,
            border_width,
            escape_js(&uri_path),
            n.line,
            n.col,
        ));
        if i + 1 < data.nodes.len() {
            nodes_json.push(',');
        }
    }
    nodes_json.push(']');

    // Build edges JSON array for the template's application/json blocks.
    let mut edges_json = String::from("[");
    for (i, e) in data.edges.iter().enumerate() {
        edges_json.push_str(&format!(
            r##"{{"from":{},"to":{},"label":"{}","arrows":"to","color":{{"color":"#475569","highlight":"#94a3b8","hover":"#94a3b8"}},"font":{{"face":"system-ui, sans-serif","color":"#94a3b8","size":11,"background":"#0f0f11","strokeWidth":0}}}}"##,
            e.from,
            e.to,
            escape_js(&e.label),
        ));
        if i + 1 < data.edges.len() {
            edges_json.push(',');
        }
    }
    edges_json.push(']');

    let vis_network_js = include_str!("../assets/vis-network.min.js");
    let vis_network_js_base64 = STANDARD.encode(vis_network_js.as_bytes());
    let template = include_str!("../assets/template.html");

    template
        .replace("{{center_name}}", &center_name)
        .replace("{{vis_network_js_base64}}", &vis_network_js_base64)
        .replace("{{nodes_json}}", &nodes_json)
        .replace("{{edges_json}}", &edges_json)
        .replace("{{center_id}}", &data.center_symbol.id.to_string())
}

/// Write HTML to .link/show.html and open in default browser.
pub fn open_graph(link_dir: &Path, data: &GraphData) -> Result<()> {
    let repo_root = link_dir.parent().unwrap_or(Path::new("."));
    let html = generate_html(data, repo_root);
    let html_path = link_dir.join("show.html");
    std::fs::write(&html_path, &html)
        .with_context(|| format!("failed to write {}", html_path.display()))?;
    open::that(&html_path).with_context(|| format!("failed to open {}", html_path.display()))?;
    Ok(())
}

fn kind_color(kind: &str) -> &'static str {
    match kind {
        "function" => "#61afef",
        "class" => "#c678dd",
        "method" => "#56b6c2",
        "variable" => "#d19a66",
        "call" => "#e06c75",
        "import" => "#98c379",
        "struct" | "enum" | "type" | "interface" => "#e5c07b",
        _ => "#abb2bf",
    }
}

fn short_path(path: &str) -> &str {
    path.rsplit_once(['/', '\\'])
        .map(|(_, f)| f)
        .unwrap_or(path)
}

fn escape_json(s: &str) -> String {
    let mut escaped = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            '<' => escaped.push_str("\\u003C"),
            '>' => escaped.push_str("\\u003E"),
            '&' => escaped.push_str("\\u0026"),
            '\u{2028}' => escaped.push_str("\\u2028"),
            '\u{2029}' => escaped.push_str("\\u2029"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn escape_js(s: &str) -> String {
    escape_json(s)
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn safe_repo_file_path(repo_root: &Path, relative_path: &str) -> Option<String> {
    let relative_path = Path::new(relative_path);
    if relative_path.is_absolute() {
        return None;
    }

    if relative_path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return None;
    }

    let absolute_path = repo_root.join(relative_path);
    if !absolute_path.starts_with(repo_root) {
        return None;
    }

    let mut path = absolute_path.to_string_lossy().to_string();
    if path.starts_with(r"\\?\UNC\") {
        path = format!(r"\\{}", &path[8..]);
    } else if path.starts_with(r"\\?\") {
        path = path[4..].to_string();
    }

    Some(path.replace('\\', "/"))
}
