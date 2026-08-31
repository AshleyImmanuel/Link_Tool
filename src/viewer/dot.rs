use super::graph::GraphData;

/// Serialize graph data to Graphviz DOT format.
pub fn graph_to_dot(data: &GraphData) -> String {
    let mut dot = String::from("digraph LinkMap {\n");
    dot.push_str("  node [shape=box, style=filled, fontname=\"Helvetica,Arial,sans-serif\"];\n");
    dot.push_str("  edge [fontname=\"Helvetica,Arial,sans-serif\", fontsize=10];\n\n");

    for node in &data.nodes {
        let color = if node.is_center {
            "\"#FFD700\""
        } else if node.is_changed {
            "\"#fb7185\""
        } else {
            "\"#e2e8f0\"" // Light gray for normal nodes in DOT
        };
        
        let label = escape_dot(&node.label);
        let kind = escape_dot(&node.kind);
        let title = format!("{}\\n{}", label, kind);
        
        dot.push_str(&format!(
            "  n{} [label=\"{}\", fillcolor={}];\n",
            node.id, title, color
        ));
    }
    
    dot.push_str("\n");
    
    for edge in &data.edges {
        let color = if edge.changed { "\"#fbbf24\"" } else { "\"#64748b\"" };
        let label = escape_dot(&edge.label);
        
        dot.push_str(&format!(
            "  n{} -> n{} [label=\"{}\", color={}];\n",
            edge.from, edge.to, label, color
        ));
    }

    dot.push_str("}\n");
    dot
}

fn escape_dot(s: &str) -> String {
    s.replace('"', "\\\"").replace('\n', "\\n")
}
