use anyhow::{Context, Result};

use crate::db::Db;
use crate::error::user_error;
use crate::intel;
use crate::ui;
use crate::viewer;

pub fn run(
    symbol_name: &str,
    dot: bool,
    out_path: Option<&str>,
    quiet: bool,
) -> Result<()> {
    let cwd = std::env::current_dir().context("failed to get current directory")?;
    let link_dir = cwd.join(".link");

    if !link_dir.join("index.db").exists() {
        return Err(user_error("not a Link project. Run 'linkmap init' first."));
    }

    let db = Db::open_index(&link_dir)?;
    let symbols = db.find_symbols_by_name(symbol_name)?;

    let defs: Vec<_> = symbols
        .iter()
        .filter(|s| intel::is_definition_kind(&s.kind))
        .collect();

    if defs.is_empty() {
        return Err(user_error(format!("symbol '{}' not found.", symbol_name)));
    }

    let target = defs[0];
    let graph = viewer::build_graph(&db, target, &cwd)?;

    let output_str = if dot {
        viewer::graph_to_dot(&graph)
    } else {
        viewer::graph_to_json(&graph)
    };

    if let Some(path) = out_path {
        std::fs::write(path, &output_str)
            .with_context(|| format!("failed to write {}", path))?;
        if !quiet {
            ui::info(quiet, format!("Exported graph to {}", path));
        }
    } else {
        println!("{}", output_str);
    }

    Ok(())
}
