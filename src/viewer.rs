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
pub fn generate_html(data: &GraphData) -> String {
    let _json = graph_to_json(data);
    let center_name = escape_html(&data.center_symbol.name);

    // Build nodes JS array
    let mut nodes_js = String::from("[");
    for (i, n) in data.nodes.iter().enumerate() {
        let color = kind_color(&n.kind);
        let border = if n.is_center { "#FFD700" } else { color };
        let border_width = if n.is_center { 3 } else { 1 };
        let file_escaped = n.file.replace('\\', "/");
        nodes_js.push_str(&format!(
            r##"{{id:{},label:"{}",color:{{background:"{}",border:"{}"}},borderWidth:{},font:{{color:"#e0e0e0",size:12}},shape:"box",file:"{}",line:{},col:{}}}"##,
            n.id,
            escape_js(&n.label),
            color,
            border,
            border_width,
            escape_js(&file_escaped),
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
            r##"{{from:{},to:{},label:"{}",arrows:"to",color:{{color:"#666",highlight:"#aaa"}},font:{{color:"#999",size:10}}}}"##,
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
  body {{ background: #1a1a2e; color: #e0e0e0; font-family: 'Segoe UI', system-ui, sans-serif; height: 100vh; overflow: hidden; }}
  #header {{ background: #16213e; padding: 12px 20px; display: flex; align-items: center; justify-content: space-between; border-bottom: 1px solid #333; height: 48px; }}
  #header h1 {{ font-size: 16px; font-weight: 600; }}
  #header h1 span {{ color: #FFD700; }}
  #legend {{ display: flex; gap: 12px; font-size: 11px; }}
  .legend-item {{ display: flex; align-items: center; gap: 4px; }}
  .legend-dot {{ width: 10px; height: 10px; border-radius: 2px; }}
  #graph {{ position: absolute; top: 48px; bottom: 0; left: 0; right: 0; }}
  #tooltip {{ position: absolute; display: none; background: #16213e; border: 1px solid #444; padding: 8px 12px; border-radius: 4px; font-size: 12px; pointer-events: none; z-index: 100; }}
</style>
</head>
<body>
<div id="header">
  <h1>🔗 Link — <span>{center_name}</span></h1>
  <div id="legend">
    <div class="legend-item"><div class="legend-dot" style="background:#4a9eff"></div>function</div>
    <div class="legend-item"><div class="legend-dot" style="background:#b266ff"></div>class</div>
    <div class="legend-item"><div class="legend-dot" style="background:#4dd0b8"></div>method</div>
    <div class="legend-item"><div class="legend-dot" style="background:#ff9f43"></div>variable</div>
    <div class="legend-item"><div class="legend-dot" style="background:#ff6b6b"></div>call</div>
    <div class="legend-item"><div class="legend-dot" style="background:#666"></div>other</div>
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
  layout: {{ hierarchical: {{ direction: 'LR', sortMethod: 'directed', levelSeparation: 200, nodeSpacing: 80 }} }},
  physics: {{ enabled: false }},
  interaction: {{ hover: true, navigationButtons: true, keyboard: true, zoomView: true }},
  edges: {{ smooth: {{ type: 'cubicBezier' }} }}
}};
var network = new vis.Network(container, data, options);
network.focus({center_id}, {{ scale: 1.0, animation: true }});
network.on('click', function(params) {{
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
  tip.innerHTML = '<b>' + node.label + '</b><br>Click to open in editor';
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
        "function" => "#3b6ea5",
        "class" => "#825ab4",
        "method" => "#3b8c7c",
        "variable" => "#b46e32",
        "call" => "#a54b4b",
        "import" => "#5c7aa5",
        "struct" | "enum" | "type" | "interface" => "#8f6b9e",
        _ => "#555555",
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
