use anyhow::{bail, Context, Result};

use crate::db::Db;

pub fn run(symbol_name: &str) -> Result<()> {
    let cwd = std::env::current_dir().context("failed to get current directory")?;
    let link_dir = cwd.join(".link");

    if !link_dir.join("index.db").exists() {
        bail!("not a Link project. Run 'link init' first.");
    }

    let db = Db::open(&link_dir)?;
    let symbols = db.find_symbols_by_name(symbol_name)?;

    // Filter to definitions only
    let defs: Vec<_> = symbols
        .iter()
        .filter(|s| s.kind != "call" && s.kind != "import")
        .collect();

    if defs.is_empty() {
        // Try fuzzy
        let fuzzy = db.fuzzy_search(symbol_name)?;
        let fuzzy_defs: Vec<_> = fuzzy
            .iter()
            .filter(|s| s.kind != "call" && s.kind != "import")
            .take(5)
            .collect();

        if fuzzy_defs.is_empty() {
            bail!("symbol '{}' not found.", symbol_name);
        }

        println!("No exact match for '{}'. Did you mean:", symbol_name);
        for s in &fuzzy_defs {
            println!("  {} ({}) {}:{}", s.name, s.kind, s.file, s.line);
        }
        return Ok(());
    }

    for target in &defs {
        println!("🔍 {} ({}) — {}:{}", target.name, target.kind, target.file, target.line);
        println!();

        let edges_and_syms = db.edges_for_symbol(target.id)?;

        // Categorize edges
        let mut called_by = Vec::new();
        let mut calls = Vec::new();
        let mut imported_by = Vec::new();
        let mut uses = Vec::new();

        for (edge, other) in &edges_and_syms {
            match edge.relation.as_str() {
                "calls" if edge.from_id == target.id => calls.push(other),
                "calls" if edge.to_id == target.id => called_by.push(other),
                "imports" if edge.from_id != target.id => imported_by.push(other),
                "imports" if edge.from_id == target.id => uses.push(other),
                _ => {}
            }
        }

        if !called_by.is_empty() {
            println!("  Called by:");
            for s in &called_by {
                println!("    {}:{}  → {}()", s.file, s.line, s.name);
            }
            println!();
        }

        if !calls.is_empty() {
            println!("  Calls:");
            for s in &calls {
                println!("    {}:{}  → {}()", s.file, s.line, s.name);
            }
            println!();
        }

        if !imported_by.is_empty() {
            println!("  Imported by:");
            for s in &imported_by {
                println!("    {}:{}  → {}", s.file, s.line, s.name);
            }
            println!();
        }

        if !uses.is_empty() {
            println!("  Uses:");
            for s in &uses {
                println!("    {}:{}  → {}", s.file, s.line, s.name);
            }
            println!();
        }

        if called_by.is_empty() && calls.is_empty() && imported_by.is_empty() && uses.is_empty() {
            println!("  No connections found.");
            println!();
        }
    }

    Ok(())
}
