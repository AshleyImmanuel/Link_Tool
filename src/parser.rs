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

