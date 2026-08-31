use super::graph::GraphData;

/// Serialize graph data to JSON string.
pub fn graph_to_json(data: &GraphData) -> String {
    let center = serde_json::json!({
        "id": data.center_symbol.id,
        "name": data.center_symbol.name,
        "kind": data.center_symbol.kind,
        "file": data.center_symbol.file,
        "line": data.center_symbol.line,
    });

    let mut nodes = Vec::new();
    for node in &data.nodes {
        nodes.push(serde_json::json!({
            "id": node.id,
            "label": node.label,
            "kind": node.kind,
            "title": node.title,
            "file": node.file,
            "line": node.line,
            "col": node.col,
            "center": node.is_center,
            "changed": node.is_changed,
            "impact_depth": node.impact_depth,
        }));
    }

    let mut edges = Vec::new();
    for edge in &data.edges {
        edges.push(serde_json::json!({
            "from": edge.from,
            "to": edge.to,
            "label": edge.label,
            "title": edge.title,
            "changed": edge.changed,
        }));
    }

    let result = serde_json::json!({
        "center": center,
        "nodes": nodes,
        "edges": edges,
    });

    serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string())
}
