use std::path::PathBuf;
use std::time::Instant;

use anyhow::{Context, Result};
use walkdir::WalkDir;

use crate::db::Db;
use crate::extractor;
use crate::hasher;
use crate::lang;
use crate::resolver;

pub fn run(quiet: bool) -> Result<()> {
    let start = Instant::now();
    let cwd = std::env::current_dir().context("failed to get current directory")?;
    let link_dir = cwd.join(".link");

    let db = Db::open(&link_dir)?;

    // Collect files
    let mut files: Vec<(PathBuf, lang::Lang)> = Vec::new();
    for entry in WalkDir::new(&cwd)
        .follow_links(true)
        .into_iter()
        .filter_entry(|e| {
            if e.file_type().is_dir() {
                let name = e.file_name().to_str().unwrap_or("");
                !lang::should_skip_dir(name)
            } else {
                true
            }
        })
    {
        let entry = match entry {
            Ok(e) => e,
            Err(err) => {
                if !quiet {
                    eprintln!("warning: {}", err);
                }
                continue;
            }
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.into_path();
        if let Some(l) = lang::detect_lang(&path) {
            if lang::is_too_large(&path) {
                if !quiet {
                    eprintln!("warning: skipping large file: {}", path.display());
                }
                continue;
            }
            files.push((path, l));
        }
    }

    if files.is_empty() {
        println!("No supported files found.");
        return Ok(());
    }

    if !quiet && files.len() > 10_000 {
        eprintln!("warning: {} files found, this may take a while...", files.len());
    }

    // Parse and extract
    let mut total_symbols = 0u64;
    for (path, l) in &files {
        let rel = path
            .strip_prefix(&cwd)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");

        let source = match std::fs::read(path) {
            Ok(s) => s,
            Err(err) => {
                if !quiet {
                    eprintln!("warning: cannot read {}: {}", rel, err);
                }
                continue;
            }
        };

        // Check UTF-8
        if std::str::from_utf8(&source).is_err() {
            if !quiet {
                eprintln!("warning: skipping non-UTF8 file: {}", rel);
            }
            continue;
        }

        let hash = hasher::hash_bytes(&source);
        let modified = path
            .metadata()
            .and_then(|m| m.modified())
            .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs() as i64)
            .unwrap_or(0);

        // Extract symbols
        let extracts = match extractor::extract(&source, *l) {
            Ok(e) => e,
            Err(err) => {
                if !quiet {
                    eprintln!("warning: parse error in {}: {}", rel, err);
                }
                continue;
            }
        };

        // Insert symbols
        for sym in &extracts.symbols {
            db.insert_symbol(&sym.name, &sym.kind, &rel, sym.line, sym.col, sym.byte_start, sym.byte_end)?;
            total_symbols += 1;
        }

        // Insert calls as "call" kind symbols (for resolver)
        for call in &extracts.calls {
            db.insert_symbol(&call.callee_name, "call", &rel, call.line, call.col, 0, 0)?;
            total_symbols += 1;
        }

        // Insert imports as "import" kind symbols
        for imp in &extracts.imports {
            db.insert_symbol(&imp.imported_name, "import", &rel, imp.line, 0, 0, 0)?;
        }

        db.upsert_file(&rel, &hash, l.name(), modified)?;
    }

    // Resolve cross-file edges
    let resolve_stats = resolver::resolve(&db)?;

    let elapsed = start.elapsed();
    let edge_count = db.edge_count()?;

    if !quiet {
        println!(
            "Initialized .link/index.db\n  {} files | {} symbols | {} edges | {:.1}s\n  ({} calls resolved, {} ambiguous, {} imports linked)",
            files.len(),
            total_symbols,
            edge_count,
            elapsed.as_secs_f64(),
            resolve_stats.resolved,
            resolve_stats.ambiguous,
            resolve_stats.imports,
        );
    }

    db.set_meta("last_scan", &chrono_now())?;
    Ok(())
}

fn chrono_now() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    now.to_string()
}
