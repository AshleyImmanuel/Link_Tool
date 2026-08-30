use anyhow::Result;
use std::path::Path;

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
        label: format!("{} ({}:{})", symbol.name, short_path(&symbol.file), symbol.line),
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
                label: format!("{} ({}:{})", other.name, short_path(&other.file), other.line),
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
            if i + 1 < data.edges.len() { ",\n" } else { "\n" }
        ));
    }
    json.push_str("  ]\n");
    json.push('}');
    json
}

/// Generate self-contained HTML with embedded vis-network for the graph.
pub fn generate_html(data: &GraphData, repo_root: &Path) -> String {
    let _json = graph_to_json(data);
    let center_name = escape_html(&data.center_symbol.name);

    // Build nodes JS array
    let mut nodes_js = String::from("[");
    for (i, n) in data.nodes.iter().enumerate() {
        let type_color = kind_color(&n.kind);
        let bg_color = "#282c34"; // Dark node background
        let border_color = if n.is_center { "#FFD700" } else { type_color };
        let border_width = if n.is_center { 3 } else { 2 };
        
        // Resolve absolute path for VS Code uri scheme
        let mut abs_path = match repo_root.canonicalize() {
            Ok(root) => root.join(&n.file).to_string_lossy().to_string(),
            Err(_) => repo_root.join(&n.file).to_string_lossy().to_string(),
        };

        // canonicalize() on Windows returns UNC paths `\\?\D:\...`
        if abs_path.starts_with(r"\\?\") {
            abs_path = abs_path[4..].to_string();
        } else if abs_path.starts_with(r"\\?\UNC\") {
            abs_path = format!(r"\\{}", &abs_path[8..]);
        }
        
        let abs_path = abs_path.replace('\\', "/");
        // VS Code on Windows often expects a leading slash before the drive letter for file URIs
        let uri_path = if abs_path.chars().nth(1) == Some(':') {
            format!("/{}", abs_path)
        } else {
            abs_path.clone()
        };

        nodes_js.push_str(&format!(
            r##"{{id:{},label:"{}",color:{{background:"#1e1e24",border:"{}",highlight:{{background:"#2a2a35",border:"{}"}},hover:{{background:"#2a2a35",border:"{}"}}}},borderWidth:{},font:{{face:"ui-monospace, SFMono-Regular, Consolas, monospace",color:"#e2e8f0",size:13}},shape:"box",margin:10,shadow:{{enabled:true,color:"rgba(0,0,0,0.5)",size:5,x:2,y:2}},file:"{}",line:{},col:{}}}"##,
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
            nodes_js.push(',');
        }
    }
    nodes_js.push(']');

    // Build edges JS array
    let mut edges_js = String::from("[");
    for (i, e) in data.edges.iter().enumerate() {
        edges_js.push_str(&format!(
            r##"{{from:{},to:{},label:"{}",arrows:"to",color:{{color:"#475569",highlight:"#94a3b8",hover:"#94a3b8"}},font:{{face:"system-ui, sans-serif",color:"#94a3b8",size:11,background:"#0f0f11",strokeWidth:0}}}}"##,
            e.from,
            e.to,
            escape_js(&e.label),
        ));
        if i + 1 < data.edges.len() {
            edges_js.push(',');
        }
    }
    edges_js.push(']');

    let vis_network_js = include_str!("../assets/vis-network.min.js");

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Link — {center_name}</title>
<script>{vis_network_js}</script>
<style>
  * {{ margin: 0; padding: 0; box-sizing: border-box; }}
  body {{ background: #0f0f11; color: #e2e8f0; font-family: system-ui, -apple-system, sans-serif; height: 100vh; overflow: hidden; }}
  #header {{ 
    position: absolute; top: 0; left: 0; right: 0; height: 56px; z-index: 10;
    background: rgba(15, 15, 17, 0.7); backdrop-filter: blur(12px); -webkit-backdrop-filter: blur(12px);
    border-bottom: 1px solid rgba(255,255,255,0.08);
    display: flex; align-items: center; justify-content: space-between; padding: 0 24px;
  }}
  #header h1 {{ font-size: 15px; font-weight: 500; letter-spacing: 0.5px; }}
  #header h1 strong {{ color: #ffffff; font-weight: 700; }}
  #header h1 span {{ color: #38bdf8; font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace; }}
  #legend {{ display: flex; gap: 16px; font-size: 12px; font-weight: 500; color: #94a3b8; }}
  .legend-item {{ display: flex; align-items: center; gap: 6px; }}
  .legend-dot {{ width: 10px; height: 10px; border-radius: 50%; }}
  #graph {{ position: absolute; top: 0; bottom: 0; left: 0; right: 0; z-index: 1; }}
  #tooltip {{ 
    position: absolute; display: none; z-index: 100; pointer-events: none;
    background: rgba(30,30,36,0.95); backdrop-filter: blur(8px);
    border: 1px solid rgba(255,255,255,0.1); border-radius: 6px;
    padding: 10px 14px; color: #e2e8f0;
    box-shadow: 0 4px 12px rgba(0,0,0,0.5);
  }}
  #tooltip b {{ color: #fff; font-family: ui-monospace, SFMono-Regular, Consolas, monospace; font-size: 13px; display: block; margin-bottom: 4px; }}
  #tooltip span {{ font-size: 12px; color: #94a3b8; }}
</style>
</head>
<body>
<div id="header">
  <h1><strong>Link</strong> <span style="color:#666">/</span> <span>{center_name}</span></h1>
  <div id="legend">
    <div class="legend-item"><div class="legend-dot" style="background:#61afef"></div>function</div>
    <div class="legend-item"><div class="legend-dot" style="background:#c678dd"></div>class/struct</div>
    <div class="legend-item"><div class="legend-dot" style="background:#56b6c2"></div>method</div>
    <div class="legend-item"><div class="legend-dot" style="background:#d19a66"></div>variable</div>
    <div class="legend-item"><div class="legend-dot" style="background:#e06c75"></div>call</div>
    <div class="legend-item"><div class="legend-dot" style="background:#abb2bf"></div>other</div>
  </div>
</div>
<div id="graph"></div>
<div id="tooltip"></div>
<script>
var nodes = new vis.DataSet({nodes_js});
var edges = new vis.DataSet({edges_js});
var container = document.getElementById('graph');
var data = {{ nodes: nodes, edges: edges }};
var options = {{
  layout: {{ hierarchical: {{ direction: 'LR', sortMethod: 'directed', levelSeparation: 250, nodeSpacing: 100 }} }},
  physics: {{ enabled: false }},
  interaction: {{ hover: true, navigationButtons: true, keyboard: true, zoomView: true }},
  edges: {{ smooth: {{ type: 'cubicBezier', roundness: 0.6 }} }}
}};
var network = new vis.Network(container, data, options);
network.focus({center_id}, {{ scale: 1.0, animation: true }});
network.on('doubleClick', function(params) {{
  if (params.nodes.length > 0) {{
    var node = nodes.get(params.nodes[0]);
    if (node && node.file) {{
      window.open('vscode://file/' + node.file + ':' + node.line + ':' + (node.col || 0), '_self');
    }}
  }}
}});
network.on('hoverNode', function(params) {{
  var node = nodes.get(params.node);
  var tip = document.getElementById('tooltip');
  tip.innerHTML = '<b>' + node.label + '</b><span>Click to open in editor</span>';
  tip.style.display = 'block';
  tip.style.left = params.event.center.x + 15 + 'px';
  tip.style.top = params.event.center.y + 15 + 'px';
}});
network.on('blurNode', function() {{
  document.getElementById('tooltip').style.display = 'none';
}});
</script>
</body>
</html>"##,
        center_name = center_name,
        nodes_js = nodes_js,
        edges_js = edges_js,
        center_id = data.center_symbol.id,
    )
}

/// Write HTML to .link/show.html and open in default browser.
pub fn open_graph(link_dir: &Path, data: &GraphData) -> Result<()> {
    let html = generate_html(data);
    let html_path = link_dir.join("show.html");
    std::fs::write(&html_path, &html)?;
    open::that(&html_path)?;
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
    path.rsplit_once(['/', '\\']).map(|(_, f)| f).unwrap_or(path)
}

fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn escape_js(s: &str) -> String {
    escape_json(s)
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
