use anyhow::{Context, Result};

use crate::db::Db;
use crate::error::user_error;
use crate::intel;

pub fn run() -> Result<()> {
    let cwd = std::env::current_dir().context("failed to get current directory")?;
    let link_dir = cwd.join(".link");
    let db_path = link_dir.join("index.db");

    if !db_path.exists() {
        return Err(user_error("not a Link project. Run 'linkmap init' first."));
    }

    let db = Db::open_index(&link_dir)?;
    let files = db.file_count()?;
    let symbols = db.symbol_count()?;
    let edges = db.edge_count()?;
    let last_scan = db.get_meta("last_scan")?;
    let db_size = std::fs::metadata(&db_path)
        .map(|metadata| metadata.len())
        .unwrap_or(0);

    let all_symbols = db.list_all_symbols()?;
    let routes = all_symbols
        .iter()
        .filter(|symbol| intel::is_definition_kind(&symbol.kind))
        .filter(|symbol| intel::semantic_kind(symbol) == "route")
        .count();
    let components = all_symbols
        .iter()
        .filter(|symbol| intel::is_definition_kind(&symbol.kind))
        .filter(|symbol| intel::semantic_kind(symbol) == "component")
        .count();
    let handlers = all_symbols
        .iter()
        .filter(|symbol| intel::is_definition_kind(&symbol.kind))
        .filter(|symbol| intel::semantic_kind(symbol) == "handler")
        .count();

    println!("Link Index Stats");
    println!("  Files:      {}", files);
    println!("  Symbols:    {}", symbols);
    println!("  Edges:      {}", edges);
    println!("  Routes:     {}", routes);
    println!("  Components: {}", components);
    println!("  Handlers:   {}", handlers);

    if let Some(timestamp) = last_scan {
        if let Ok(epoch) = timestamp.parse::<u64>() {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let ago = now.saturating_sub(epoch);
            println!("  Last scan:  {}", format_duration(ago));
        }
    }

    println!("  DB size:    {}", format_size(db_size));
    println!();

    let violations = intel::architecture_violations(&cwd, &db)?;
    if violations.is_empty() {
        println!("Architecture Rules");
        println!("  Scope: built-in heuristic import checks, not a configurable policy engine.");
        println!("  No violations detected.");
        println!();
    } else {
        println!("Architecture Rules");
        println!("  Scope: built-in heuristic import checks, not a configurable policy engine.");
        for violation in violations.iter().take(8) {
            println!(
                "  {}:{} [{}] {} -> {}",
                violation.file,
                violation.line,
                violation.rule,
                violation.import_target,
                violation.detail
            );
        }
        if violations.len() > 8 {
            println!("  ... and {} more", violations.len() - 8);
        }
        println!();
    }

    if let Some(summary) = intel::collect_change_summary(&cwd)? {
        println!("Change Summary");
        println!("  Scope: local git working tree vs HEAD only; not remote or push aware.");
        println!("  Diff: extracted symbol/import/call/render signatures, not full semantic diff.");
        println!("  Changed files: {}", summary.changed_files.len());
        print_items("Changed", &summary.changed_files);
        print_items("Added symbols", &summary.added_symbols);
        print_items("Removed symbols", &summary.removed_symbols);
        print_items("Added edges", &summary.added_edges);
        print_items("Removed edges", &summary.removed_edges);
    }

    Ok(())
}

fn print_items(title: &str, items: &[String]) {
    if items.is_empty() {
        return;
    }

    const MAX_PRINTED: usize = 12;

    println!("  {}:", title);
    for item in items.iter().take(MAX_PRINTED) {
        println!("    {}", item);
    }
    if items.len() > MAX_PRINTED {
        println!("    ... and {} more", items.len() - MAX_PRINTED);
    }
}

fn format_duration(secs: u64) -> String {
    if secs < 60 {
        format!("{} seconds ago", secs)
    } else if secs < 3600 {
        format!("{} minutes ago", secs / 60)
    } else if secs < 86400 {
        format!("{} hours ago", secs / 3600)
    } else {
        format!("{} days ago", secs / 86400)
    }
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1_048_576 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    }
}
