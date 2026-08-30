use anyhow::{Context, Result};
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
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

/// A raw render/reference extracted from JSX/TSX.
#[derive(Debug, Clone)]
pub struct RawRender {
    pub component_name: String,
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

/// A raw route extracted from a single file (Express/Laravel/etc).
#[derive(Debug, Clone)]
pub struct RawRoute {
    pub method: String,
    pub path: String,
    pub handler_name: String,
    pub line: u32,
    pub col: u32,
}

/// Result of extracting symbols, calls, and imports from a single file.
#[derive(Debug, Default)]
pub struct FileExtracts {
    pub symbols: Vec<RawSymbol>,
    pub calls: Vec<RawCall>,
    pub renders: Vec<RawRender>,
    pub imports: Vec<RawImport>,
    pub routes: Vec<RawRoute>,
}

#[derive(Default)]
pub struct ExtractorPool {
    extractors: HashMap<Lang, LanguageExtractor>,
}

struct LanguageExtractor {
    parser: tree_sitter::Parser,
    query: Query,
    capture_names: Vec<String>,
}

impl ExtractorPool {
    /// Extract all symbols, calls, and imports from source code.
    pub fn extract(&mut self, source: &[u8], lang: Lang) -> Result<FileExtracts> {
        let extractor = match self.extractors.entry(lang) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(LanguageExtractor::new(lang)?),
        };

        let tree = parser::parse_with(&mut extractor.parser, source, lang)
            .with_context(|| format!("failed to parse as {}", lang.name()))?;

        let mut cursor = tree_sitter::QueryCursor::new();
        let mut matches = cursor.matches(&extractor.query, tree.root_node(), source);
        let mut result = FileExtracts::default();

        while let Some(m) = matches.next() {
            for capture in m.captures {
                let idx = capture.index as usize;
                let Some(name) = extractor.capture_names.get(idx) else {
                    continue;
                };
                let node = capture.node;
                let start = node.start_position();
                let text = node_text(source, node);

                if text.is_empty() {
                    continue;
                }

                if let Some(kind) = name.strip_prefix("definition.") {
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
                } else if name == "render" {
                    result.renders.push(RawRender {
                        component_name: text,
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
                } else if name == "route.method" {
                    result.routes.push(RawRoute {
                        method: text.to_ascii_uppercase(),
                        path: String::new(),
                        handler_name: String::new(),
                        line: start.row as u32 + 1,
                        col: start.column as u32,
                    });
                } else if name == "route.path" {
                    let text = text.trim_matches(|c| c == '\'' || c == '"').to_string();
                    if let Some(route) = result.routes.last_mut() {
                        if route.path.is_empty() && route.line == start.row as u32 + 1 {
                            route.path = text;
                        }
                    }
                } else if name == "route.handler" {
                    if let Some(route) = result.routes.last_mut() {
                        if route.handler_name.is_empty() && route.line == start.row as u32 + 1 {
                            route.handler_name =
                                text.trim_matches(|c| c == '\'' || c == '"').to_string();
                        }
                    }
                }
            }
        }

        result.symbols = dedupe_symbols(result.symbols);
        result
            .routes
            .retain(|r| !r.method.is_empty() && !r.path.is_empty() && !r.handler_name.is_empty());
        Ok(result)
    }
}

impl LanguageExtractor {
    fn new(lang: Lang) -> Result<Self> {
        let parser = parser::new_parser(lang)?;
        let query_src = parser::query_str(lang);
        let query = Query::new(&lang.ts_language(), query_src)
            .with_context(|| format!("failed to load tree-sitter query for {}", lang.name()))?;
        let capture_names = query
            .capture_names()
            .iter()
            .map(|capture| capture.to_string())
            .collect();

        Ok(Self {
            parser,
            query,
            capture_names,
        })
    }
}

fn node_text(source: &[u8], node: tree_sitter::Node) -> String {
    let bytes = &source[node.start_byte()..node.end_byte()];
    String::from_utf8_lossy(bytes).to_string()
}

fn dedupe_symbols(symbols: Vec<RawSymbol>) -> Vec<RawSymbol> {
    let function_sites: HashSet<(String, u32, u32, u32, u32)> = symbols
        .iter()
        .filter(|symbol| symbol.kind == "function")
        .map(|symbol| {
            (
                symbol.name.clone(),
                symbol.line,
                symbol.col,
                symbol.byte_start,
                symbol.byte_end,
            )
        })
        .collect();

    symbols
        .into_iter()
        .filter(|symbol| {
            if symbol.kind != "variable" {
                return true;
            }

            !function_sites.contains(&(
                symbol.name.clone(),
                symbol.line,
                symbol.col,
                symbol.byte_start,
                symbol.byte_end,
            ))
        })
        .collect()
}
