use anyhow::{Context, Result};

use crate::db::Db;
use crate::error::user_error;
use crate::intel;

pub fn run(quiet: bool) -> Result<()> {
    let cwd = std::env::current_dir().context("failed to get current directory")?;
    let link_dir = cwd.join(".link");

    if !link_dir.join("index.db").exists() {
        return Err(user_error("not a Link project. Run 'linkmap init' first."));
    }

    let db = Db::open_index(&link_dir)?;
    let symbols = db.list_all_symbols()?;

    if symbols.is_empty() {
        println!("No symbols indexed.");
        return Ok(());
    }

    // Filter out calls and imports for clean output
    let defs: Vec<_> = symbols
        .iter()
        .filter(|s| intel::is_definition_kind(&s.kind))
        .collect();

    if !quiet {
        println!("{:<30} {:<12} LOCATION", "NAME", "KIND");
        println!("{}", "-".repeat(70));
    }

    for s in &defs {
        if quiet {
            println!("{}", s.name);
        } else {
            println!("{:<30} {:<12} {}:{}", s.name, s.kind, s.file, s.line);
        }
    }

    if !quiet {
        println!("\n{} symbols", defs.len());
    }

    Ok(())
}
