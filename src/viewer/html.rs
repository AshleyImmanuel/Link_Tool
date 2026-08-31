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

    let mut nodes_json_vec = Vec::new();
    for node in data.nodes.iter() {
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

        nodes_json_vec.push(serde_json::json!({
            "id": node.id,
            "label": node.label,
            "kind": node.kind,
            "title": node.title,
            "color": {
                "background": background,
                "border": border_color,
                "highlight": { "background": highlight_bg, "border": border_color },
                "hover": { "background": highlight_bg, "border": border_color }
            },
            "borderWidth": border_width,
            "font": {
                "face": "ui-monospace, SFMono-Regular, Consolas, monospace",
                "color": "#f8fafc",
                "size": 14,
                "strokeWidth": 3,
                "strokeColor": "#0b0b0f"
            },
            "shape": "box",
            "margin": 10,
            "shadow": {
                "enabled": true,
                "color": "rgba(0,0,0,0.55)",
                "size": 6,
                "x": 2,
                "y": 2
            },
            "file": uri_path,
            "line": node.line,
            "col": node.col,
            "changed": node.is_changed
        }));
    }
    let nodes_json = serde_json::to_string(&nodes_json_vec).unwrap_or_else(|_| "[]".to_string());

    let mut edges_json_vec = Vec::new();
    for edge in data.edges.iter() {
        let color = if edge.changed { "#fbbf24" } else { "#64748b" };
        edges_json_vec.push(serde_json::json!({
            "from": edge.from,
            "to": edge.to,
            "label": edge.label,
            "title": edge.title,
            "arrows": "to",
            "color": {
                "color": color,
                "highlight": "#cbd5e1",
                "hover": "#cbd5e1"
            },
            "font": {
                "face": "system-ui, sans-serif",
                "color": "#cbd5e1",
                "size": 11,
                "background": "#0b0b0f",
                "strokeWidth": 3,
                "strokeColor": "#0b0b0f"
            }
        }));
    }
    let edges_json = serde_json::to_string(&edges_json_vec).unwrap_or_else(|_| "[]".to_string());

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
        .replace("/* {{viewer_css}} */", &viewer_css)
        .replace("/* {{viewer_js}} */", viewer_js)
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

