use super::graph::GraphData;

/// Serialize graph data to JSON string.
pub fn graph_to_json(data: &GraphData) -> String {
    let mut json = String::from("{\n");

    json.push_str(&format!(
        "  \"center\": {{ \"id\": {}, \"name\": \"{}\", \"kind\": \"{}\", \"file\": \"{}\", \"line\": {} }},\n",
        data.center_symbol.id,
        escape_json(&data.center_symbol.name),
        escape_json(&data.center_symbol.kind),
        escape_json(&data.center_symbol.file),
        data.center_symbol.line
    ));

    json.push_str("  \"nodes\": [\n");
    for (index, node) in data.nodes.iter().enumerate() {
        json.push_str(&format!(
            "    {{ \"id\": {}, \"label\": \"{}\", \"kind\": \"{}\", \"title\": \"{}\", \"file\": \"{}\", \"line\": {}, \"col\": {}, \"center\": {}, \"changed\": {}, \"impact_depth\": {} }}{}",
            node.id,
            escape_json(&node.label),
            escape_json(&node.kind),
            escape_json(&node.title),
            escape_json(&node.file),
            node.line,
            node.col,
            node.is_center,
            node.is_changed,
            node.impact_depth,
            if index + 1 < data.nodes.len() { ",\n" } else { "\n" }
        ));
    }
    json.push_str("  ],\n");

    json.push_str("  \"edges\": [\n");
    for (index, edge) in data.edges.iter().enumerate() {
        json.push_str(&format!(
            "    {{ \"from\": {}, \"to\": {}, \"label\": \"{}\", \"title\": \"{}\", \"changed\": {} }}{}",
            edge.from,
            edge.to,
            escape_json(&edge.label),
            escape_json(&edge.title),
            edge.changed,
            if index + 1 < data.edges.len() { ",\n" } else { "\n" }
        ));
    }
    json.push_str("  ]\n");
    json.push('}');
    json
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
