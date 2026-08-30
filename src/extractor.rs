use anyhow::Result;
use streaming_iterator::StreamingIterator;
use tree_sitter::Query;

use crate::lang::Lang;
use crate::parser;

/// A raw symbol extracted from a single file.
#[derive(Debug, Clone)]
pub struct RawSymbol {
    pub name: String,
    pub kind: String,
    pub line: u32,
    pub col: u32,
    pub byte_start: u32,
    pub byte_end: u32,
}

/// A raw call/reference extracted from a single file.
#[derive(Debug, Clone)]
pub struct RawCall {
    pub callee_name: String,
    pub line: u32,
    pub col: u32,
}

/// A raw import extracted from a single file.
#[derive(Debug, Clone)]
pub struct RawImport {
    pub imported_name: String,
    pub source_module: String,
    pub line: u32,
}

/// Result of extracting symbols, calls, and imports from a single file.
#[derive(Debug, Default)]
pub struct FileExtracts {
    pub symbols: Vec<RawSymbol>,
    pub calls: Vec<RawCall>,
    pub imports: Vec<RawImport>,
}

/// Extract all symbols, calls, and imports from source code.
pub fn extract(source: &[u8], lang: Lang) -> Result<FileExtracts> {
    let tree = parser::parse(source, lang)?;
    let query_src = parser::query_str(lang);
    let query = Query::new(&lang.ts_language(), query_src)?;

    let mut cursor = tree_sitter::QueryCursor::new();
    let mut matches = cursor.matches(&query, tree.root_node(), source);

    let mut result = FileExtracts::default();

    // Get capture names
    let capture_names: Vec<String> = query.capture_names().iter().map(|s| s.to_string()).collect();

    // StreamingIterator: call advance() then get()
    while let Some(m) = matches.next() {
        for capture in m.captures {
            let idx = capture.index as usize;
            let name = if idx < capture_names.len() {
                &capture_names[idx]
            } else {
                continue;
            };
            let node = capture.node;
            let start = node.start_position();
            let text = node_text(source, node);

            if text.is_empty() {
                continue;
            }

            if name.starts_with("definition.") {
                let kind = name.strip_prefix("definition.").unwrap_or("unknown");
                result.symbols.push(RawSymbol {
                    name: text,
                    kind: kind.to_string(),
                    line: start.row as u32 + 1,
                    col: start.column as u32,
                    byte_start: node.start_byte() as u32,
                    byte_end: node.end_byte() as u32,
                });
            } else if name == "call" {
                result.calls.push(RawCall {
                    callee_name: text,
                    line: start.row as u32 + 1,
                    col: start.column as u32,
                });
            } else if name == "import.name" {
                result.imports.push(RawImport {
                    imported_name: text,
                    source_module: String::new(),
                    line: start.row as u32 + 1,
                });
            } else if name == "import.source" {
                let text = text.trim_matches(|c| c == '\'' || c == '"').to_string();
                if let Some(imp) = result.imports.last_mut() {
                    if imp.source_module.is_empty() && imp.line == start.row as u32 + 1 {
                        imp.source_module = text;
                    }
                }
            }
        }
    }

    Ok(result)
}

fn node_text(source: &[u8], node: tree_sitter::Node) -> String {
    let bytes = &source[node.start_byte()..node.end_byte()];
    String::from_utf8_lossy(bytes).to_string()
}
