use anyhow::{anyhow, Context, Result};
use tree_sitter::{Parser, Tree};

use crate::lang::Lang;

pub fn new_parser(lang: Lang) -> Result<Parser> {
    let mut parser = Parser::new();
    parser
        .set_language(&lang.ts_language())
        .with_context(|| format!("failed to set language: {}", lang.name()))?;
    Ok(parser)
}

/// Parse file content using the appropriate Tree-sitter grammar.
pub fn parse_with(parser: &mut Parser, source: &[u8], lang: Lang) -> Result<Tree> {
    parser
        .parse(source, None)
        .ok_or_else(|| anyhow!("failed to parse as {}", lang.name()))
}

/// Get the tree-sitter query string for a language (compiled in at build time).
pub fn query_str(lang: Lang) -> &'static str {
    match lang {
        Lang::JavaScript => include_str!("../queries/javascript.scm"),
        Lang::TypeScript => include_str!("../queries/typescript.scm"),
        Lang::Tsx => include_str!("../queries/tsx.scm"),
        Lang::Python => include_str!("../queries/python.scm"),
        Lang::Go => include_str!("../queries/go.scm"),
        Lang::Rust => include_str!("../queries/rust.scm"),
        Lang::Php => include_str!("../queries/php.scm"),
    }
}
