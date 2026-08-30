use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Instant;

use anyhow::{bail, Context, Result};
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

    if !link_dir.join("index.db").exists() {
        bail!("not a Link project. Run 'link init' first.");
    }

    let db = Db::open(&link_dir)?;

    // Scan current files
    let mut current_files: Vec<(PathBuf, String, lang::Lang)> = Vec::new();
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
            Err(_) => continue,
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.into_path();
        if let Some(l) = lang::detect_lang(&path) {
            if lang::is_too_large(&path) {
                continue;
            }
            let rel = path
                .strip_prefix(&cwd)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            current_files.push((path, rel, l));
        }
    }

    let current_paths: HashSet<String> = current_files.iter().map(|(_, r, _)| r.clone()).collect();
    let stored_paths: HashSet<String> = db.all_file_paths()?.into_iter().collect();

    // Classify: new, changed, deleted
    let mut new_files = Vec::new();
    let mut changed_files = Vec::new();
    let deleted: Vec<String> = stored_paths.difference(&current_paths).cloned().collect();

    for (path, rel, l) in &current_files {
        let source = match std::fs::read(path) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let hash = hasher::hash_bytes(&source);
        let stored_hash = db.get_file_hash(rel)?;

        match stored_hash {
            None => new_files.push((path.clone(), rel.clone(), *l, source, hash)),
            Some(h) if h != hash => changed_files.push((path.clone(), rel.clone(), *l, source, hash)),
            _ => {} // unchanged
        }
    }

    if new_files.is_empty() && changed_files.is_empty() && deleted.is_empty() {
        if !quiet {
            println!("Already up to date.");
        }
        return Ok(());
    }

    // Delete symbols/edges for changed and deleted files
    for rel in &deleted {
        db.delete_file(rel)?;
    }
    for (_, rel, _, _, _) in &changed_files {
        db.delete_symbols_for_file(rel)?;
    }

    // Parse and insert new + changed
    for (path, rel, l, source, hash) in new_files.iter().chain(changed_files.iter()) {
        if std::str::from_utf8(source).is_err() {
            continue;
        }

        let extracts = match extractor::extract(source, *l) {
            Ok(e) => e,
            Err(err) => {
                if !quiet {
                    eprintln!("warning: parse error in {}: {}", rel, err);
                }
                continue;
            }
        };

        for sym in &extracts.symbols {
            db.insert_symbol(&sym.name, &sym.kind, rel, sym.line, sym.col, sym.byte_start, sym.byte_end)?;
        }
        for call in &extracts.calls {
            db.insert_symbol(&call.callee_name, "call", rel, call.line, call.col, 0, 0)?;
        }
        for imp in &extracts.imports {
            db.insert_symbol(&imp.imported_name, "import", rel, imp.line, 0, 0, 0)?;
        }

        let modified = path
            .metadata()
            .and_then(|m| m.modified())
            .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs() as i64)
            .unwrap_or(0);
        db.upsert_file(rel, hash, l.name(), modified)?;
    }

    // Re-resolve all edges (simple for v1; could optimize later)
    // First clear all edges, then re-resolve
    // Actually, since we only deleted edges for changed/deleted files,
    // and resolver builds edges based on all symbols, we need to re-resolve globally.
    // For simplicity: delete all edges and re-resolve.
    db.vacuum()?; // Clean up
    let resolve_stats = resolver::resolve(&db)?;

    let elapsed = start.elapsed();

    if !quiet {
        println!(
            "Updated: +{} new, ~{} changed, -{} deleted | {:.1}s\n  {} calls resolved, {} imports linked",
            new_files.len(),
            changed_files.len(),
            deleted.len(),
            elapsed.as_secs_f64(),
            resolve_stats.resolved,
            resolve_stats.imports,
        );
    }

    db.set_meta("last_scan", &{
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .to_string()
    })?;

    Ok(())
}
