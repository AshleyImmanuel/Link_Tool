use anyhow::{Context, Result};

use crate::db::Db;
use crate::error::user_error;
use crate::history;

pub fn run(all: bool, limit: usize) -> Result<()> {
    if limit == 0 {
        return Err(user_error("history limit must be at least 1."));
    }

    let cwd = std::env::current_dir().context("failed to get current directory")?;
    let link_dir = cwd.join(".link");
    let db_path = link_dir.join("index.db");

    if !db_path.exists() {
        println!("No Link history yet for this project.");
        return Ok(());
    }

    let db = Db::open(&link_dir)?;
    let session = history::detect_session();
    let entries = db.command_history(if all { None } else { session.key.as_deref() }, limit)?;

    println!("Link Command History");
    println!("  Scope: {}", history::scope_label(all, &session));
    println!("  Limit: {}", limit);

    if entries.is_empty() {
        if all || !history::has_exact_session(&session) {
            println!("  No Link commands recorded yet.");
        } else {
            println!("  No Link commands recorded for this session yet.");
            println!("  Tip: run `link history --all` to see the full project history.");
        }
        return Ok(());
    }

    println!();

    for (index, entry) in entries.iter().rev().enumerate() {
        let status = if entry.success { "ok" } else { "error" };
        if all {
            println!(
                "  {}. {:<5} {:>8} [{}] {}",
                index + 1,
                status,
                history::format_age(entry.ts),
                history::display_session_key(&entry.session_key),
                entry.command
            );
        } else {
            println!(
                "  {}. {:<5} {:>8} {}",
                index + 1,
                status,
                history::format_age(entry.ts),
                entry.command
            );
        }
    }

    if !all && !history::has_exact_session(&session) {
        println!();
        println!(
            "  Tip: set LINK_SESSION_ID once when you open a shell for exact session-only history."
        );
    }

    Ok(())
}
