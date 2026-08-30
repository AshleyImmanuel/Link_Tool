use anyhow::{Context, Result};

use crate::db::Db;
use crate::error::user_error;
use crate::intel;
use crate::ui;
use crate::viewer;

pub fn run(symbol_name: &str, preferred_file: Option<&str>, json: bool, quiet: bool) -> Result<()> {
    let cwd = std::env::current_dir().context("failed to get current directory")?;
    let link_dir = cwd.join(".link");

    if !link_dir.join("index.db").exists() {
        return Err(user_error("not a Link project. Run 'linkmap init' first."));
    }

    let db = Db::open_index(&link_dir)?;
    let symbols = db.find_symbols_by_name(symbol_name)?;

    // Filter to definitions only (not calls/imports)
    let defs: Vec<_> = symbols
        .iter()
        .filter(|s| intel::is_definition_kind(&s.kind))
        .collect();

    if defs.is_empty() {
        // Try fuzzy search as fallback
        let fuzzy = db.fuzzy_search(symbol_name)?;
        let fuzzy_defs: Vec<_> = fuzzy
            .iter()
            .filter(|s| intel::is_definition_kind(&s.kind))
            .take(10)
            .collect();

        if fuzzy_defs.is_empty() {
            return Err(user_error(format!("symbol '{}' not found.", symbol_name)));
        }

        println!("No exact match for '{}'. Did you mean:", symbol_name);
        for s in &fuzzy_defs {
            println!("  {} ({}) {}:{}", s.name, s.kind, s.file, s.line);
        }
        return Ok(());
    }

    // If multiple definitions, show disambiguation
    if defs.len() > 1 && !json {
        println!("Multiple definitions for '{}':", symbol_name);
        for (i, s) in defs.iter().enumerate() {
            println!(
                "  [{}] {} ({}) {}:{}",
                i + 1,
                s.name,
                s.kind,
                s.file,
                s.line
            );
        }
        if let Some(file) = preferred_file {
            println!("Trying preferred file: {}", file);
        } else {
            println!("Tip: pass --file <path> to pick the right definition.");
        }
    }

    let target = match preferred_file {
        Some(file) => {
            let normalized = file.replace('\\', "/");
            match defs.iter().find(|s| s.file == normalized) {
                Some(hit) => hit,
                None => {
                    return Err(user_error(format!(
                        "no definition for '{}' found in file '{}'.",
                        symbol_name, file
                    )));
                }
            }
        }
        None => defs[0],
    };
    let graph = viewer::build_graph(&db, target, &cwd)?;

    if json {
        println!("{}", viewer::graph_to_json(&graph));
    } else {
        viewer::open_graph(&link_dir, &graph)?;
        if !quiet {
            ui::info(
                quiet,
                format!(
                    "Opened graph for '{}' ({}) - {} nodes, {} edges",
                    target.name,
                    target.kind,
                    graph.nodes.len(),
                    graph.edges.len()
                ),
            );
        }
    }

    Ok(())
}
