use anyhow::{Context, Result};

use crate::db::Db;
use crate::error::user_error;

pub fn run() -> Result<()> {
    let cwd = std::env::current_dir().context("failed to get current directory")?;
    let link_dir = cwd.join(".link");
    let db_path = link_dir.join("index.db");

    if !db_path.exists() {
        return Err(user_error("not a Link project. Run 'link init' first."));
    }

    let db = Db::open_index(&link_dir)?;

    let files = db.file_count()?;
    let symbols = db.symbol_count()?;
    let edges = db.edge_count()?;
    let last_scan = db.get_meta("last_scan")?;
    let db_size = std::fs::metadata(&db_path)
        .map(|metadata| metadata.len())
        .unwrap_or(0);

    println!("Link Index Stats");
    println!("  Files:   {}", files);
    println!("  Symbols: {}", symbols);
    println!("  Edges:   {}", edges);

    if let Some(timestamp) = last_scan {
        if let Ok(epoch) = timestamp.parse::<u64>() {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let ago = now.saturating_sub(epoch);
            println!("  Last scan: {}", format_duration(ago));
        }
    }

    println!("  DB size: {}", format_size(db_size));

    Ok(())
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
