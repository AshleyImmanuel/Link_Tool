use std::path::{Component, Path};

use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};

use super::graph::GraphData;

/// Generate self-contained HTML with embedded vis-network for the graph.
pub fn generate_html(data: &GraphData, repo_root: &Path) -> String {
    let center_name = escape_html(&data.center_symbol.name);
    let repo_root = repo_root
        .canonicalize()
        .unwrap_or_else(|_| repo_root.to_path_buf());

    let mut nodes_json = String::from("[");
    for (index, node) in data.nodes.iter().enumerate() {
        let type_color = kind_color(&node.kind);
        let uri_path = safe_repo_file_path(&repo_root, &node.file).unwrap_or_default();
        let border_width = if node.is_center { 4 } else { 2 };
        let border_color = if node.is_center {
            "#FFD700"
        } else if node.is_changed {
            "#fb7185"
        } else {
            type_color
        };
        let background = if node.is_changed {
            "#30151d"
        } else {
            "#141622"
        };
        let highlight_bg = if node.is_changed {
            "#3a1a25"
        } else {
            "#1b1e2d"
        };

        nodes_json.push_str(&format!(
            r##"{{"id":{},"label":"{}","title":"{}","color":{{"background":"{}","border":"{}","highlight":{{"background":"{}","border":"{}"}},"hover":{{"background":"{}","border":"{}"}}}},"borderWidth":{},"font":{{"face":"ui-monospace, SFMono-Regular, Consolas, monospace","color":"#f8fafc","size":14,"strokeWidth":3,"strokeColor":"#0b0b0f"}},"shape":"box","margin":10,"shadow":{{"enabled":true,"color":"rgba(0,0,0,0.55)","size":6,"x":2,"y":2}},"file":"{}","line":{},"col":{},"changed":{}}}"##,
            node.id,
            escape_js(&node.label),
            escape_js(&node.title),
            background,
            border_color,
            highlight_bg,
            border_color,
            highlight_bg,
            border_color,
            border_width,
            escape_js(&uri_path),
            node.line,
            node.col,
            node.is_changed,
        ));
        if index + 1 < data.nodes.len() {
            nodes_json.push(',');
        }
    }
    nodes_json.push(']');

    let mut edges_json = String::from("[");
    for (index, edge) in data.edges.iter().enumerate() {
        let color = if edge.changed { "#fbbf24" } else { "#64748b" };
        edges_json.push_str(&format!(
            r##"{{"from":{},"to":{},"label":"{}","title":"{}","arrows":"to","color":{{"color":"{}","highlight":"#cbd5e1","hover":"#cbd5e1"}},"font":{{"face":"system-ui, sans-serif","color":"#cbd5e1","size":11,"background":"#0b0b0f","strokeWidth":3,"strokeColor":"#0b0b0f"}}}}"##,
            edge.from,
            edge.to,
            escape_js(&edge.label),
            escape_js(&edge.title),
            color,
        ));
        if index + 1 < data.edges.len() {
            edges_json.push(',');
        }
    }
    edges_json.push(']');

    let vis_network_js = include_str!("../../assets/vis-network.min.js");
    let vis_network_js_base64 = STANDARD.encode(vis_network_js.as_bytes());
    let template = include_str!("../../assets/template.html");
    let viewer_css_base = include_str!("../../assets/viewer/base.css");
    let viewer_css_layout = include_str!("../../assets/viewer/layout.css");
    let viewer_css_components = include_str!("../../assets/viewer/components.css");
    let viewer_css = format!(
        "{}\n{}\n{}",
        viewer_css_base, viewer_css_layout, viewer_css_components
    );
    let viewer_js = include_str!("../../assets/viewer.js");

    template
        .replace("{{center_name}}", &center_name)
        .replace("{{vis_network_js_base64}}", &vis_network_js_base64)
        .replace("{{nodes_json}}", &nodes_json)
        .replace("{{edges_json}}", &edges_json)
        .replace("{{viewer_css}}", &viewer_css)
        .replace("{{viewer_js}}", viewer_js)
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
        // Modern, high-contrast palette on dark background:
        // - avoid neon greens; keep distinct hues per kind
        "component" => "#60a5fa", // blue
        "route" => "#fbbf24",     // amber
        "handler" => "#2dd4bf",   // teal
        "function" => "#a78bfa",  // violet
        "class" => "#f472b6",     // pink
        "method" => "#22c55e",    // green (kept, but less dominant in UI than before)
        "variable" => "#fb923c",  // orange
        "call" => "#fb7185",      // rose
        "render" => "#c084fc",    // purple
        "import" => "#94a3b8",    // slate (neutral)
        _ => "#abb2bf",
    }
}

fn escape_js(s: &str) -> String {
    escape_json_string(s)
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

fn escape_json_string(s: &str) -> String {
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
