use std::collections::HashSet;
use std::time::Instant;

use anyhow::{Context, Result};

use crate::db::Db;
use crate::error::user_error;
use crate::extractor::ExtractorPool;
use crate::hasher;
use crate::resolver;
use crate::scan;
use crate::ui;

pub fn run(quiet: bool) -> Result<()> {
    let start = Instant::now();
    let cwd = std::env::current_dir().context("failed to get current directory")?;
    let link_dir = cwd.join(".link");

    if !link_dir.join("index.db").exists() {
        return Err(user_error("not a Link project. Run 'linkmap init' first."));
    }

    let db = Db::open_index(&link_dir)?;
    let current_files = scan::collect_source_files(&cwd, quiet)?;
    let current_paths: HashSet<String> = current_files
        .iter()
        .map(|file| file.rel_path.clone())
        .collect();
    let stored_paths: HashSet<String> = db.all_file_paths()?.into_iter().collect();

    let mut new_files: Vec<PendingFile> = Vec::new();
    let mut changed_files: Vec<PendingFile> = Vec::new();
    let deleted: Vec<String> = stored_paths.difference(&current_paths).cloned().collect();

    for file in &current_files {
        let source = match std::fs::read(&file.abs_path) {
            Ok(source) => source,
            Err(err) => {
                if !quiet {
                    ui::warn(
                        quiet,
                        format!("failed to read file {}: {err}", file.rel_path),
                    );
                }
                continue;
            }
        };
        let hash = hasher::hash_bytes(&source);
        let stored_hash = db.get_file_hash(&file.rel_path)?;

        match stored_hash {
            None => new_files.push(PendingFile {
                file: file.clone(),
                hash,
            }),
            Some(existing) if existing != hash => changed_files.push(PendingFile {
                file: file.clone(),
                hash,
            }),
            _ => {} // unchanged
        }
    }

    if new_files.is_empty() && changed_files.is_empty() && deleted.is_empty() {
        if !quiet {
            println!("Already up to date.");
        }
        return Ok(());
    }

    let mut extractor = ExtractorPool::default();
    db.begin_transaction()?;
    let update_result = apply_updates(
        &db,
        &mut extractor,
        &new_files,
        &changed_files,
        &deleted,
        quiet,
    );
    let (indexed_new, indexed_changed, resolve_stats) = match update_result {
        Ok(result) => {
            db.commit_transaction()?;
            result
        }
        Err(err) => {
            let _ = db.rollback_transaction();
            return Err(err);
        }
    };

    let elapsed = start.elapsed();

    if !quiet {
        ui::info(
            quiet,
            format!(
            "Updated: +{} new, ~{} changed, -{} deleted | {:.1}s\n  {} calls resolved, {} renders resolved, {} imports linked",
            indexed_new,
            indexed_changed,
            deleted.len(),
            elapsed.as_secs_f64(),
            resolve_stats.resolved,
            resolve_stats.renders,
            resolve_stats.imports,
        ),
        );
        if resolve_stats.ambiguous > 0 {
            ui::warn(
                quiet,
                format!(
                    "skipped {} ambiguous matches (conservative on purpose)",
                    resolve_stats.ambiguous
                ),
            );
        }
    }

    Ok(())
}

#[derive(Debug, Clone)]
struct PendingFile {
    file: scan::SourceFile,
    hash: String,
}

fn apply_updates(
    db: &Db,
    extractor: &mut ExtractorPool,
    new_files: &[PendingFile],
    changed_files: &[PendingFile],
    deleted: &[String],
    quiet: bool,
) -> Result<(usize, usize, resolver::ResolveStats)> {
    for rel_path in deleted {
        db.delete_file(rel_path)?;
    }

    let mut indexed_new = 0usize;
    let mut indexed_changed = 0usize;

    for pending in new_files {
        if reindex_file(db, extractor, pending, false, quiet)? {
            indexed_new += 1;
        }
    }

    for pending in changed_files {
        if reindex_file(db, extractor, pending, true, quiet)? {
            indexed_changed += 1;
        }
    }

    db.clear_edges()?;
    let resolve_stats = resolver::resolve(db)?;
    db.write_index_metadata()?;
    db.set_meta("last_scan", &chrono_now())?;
    Ok((indexed_new, indexed_changed, resolve_stats))
}

fn reindex_file(
    db: &Db,
    extractor: &mut ExtractorPool,
    pending: &PendingFile,
    replace_existing: bool,
    quiet: bool,
) -> Result<bool> {
    let source = match std::fs::read(&pending.file.abs_path) {
        Ok(source) => source,
        Err(err) => {
            if !quiet {
                eprintln!(
                    "warning: failed to read file {}: {err}",
                    pending.file.rel_path
                );
            }
            return Ok(false);
        }
    };

    if std::str::from_utf8(&source).is_err() {
        if !quiet {
            ui::warn(
                quiet,
                format!("skipping non-UTF8 file {}", pending.file.rel_path),
            );
        }
        return Ok(false);
    }

    let mut extracts = match extractor.extract(&source, pending.file.lang) {
        Ok(extracts) => extracts,
        Err(err) => {
            if !quiet {
                ui::warn(
                    quiet,
                    format!("failed to parse file {}: {err}", pending.file.rel_path),
                );
            }
            return Ok(false);
        }
    };
    if matches!(pending.file.lang, crate::lang::Lang::Php)
        && pending.file.rel_path.starts_with("routes/")
    {
        if let Ok(text) = std::str::from_utf8(&source) {
            extracts
                .routes
                .extend(crate::framework::laravel::extract_routes_from_routes_php(
                    text,
                ));
        }
    }

    if replace_existing {
        db.delete_symbols_for_file(&pending.file.rel_path)?;
    }

    for symbol in &extracts.symbols {
        db.insert_symbol(
            &symbol.name,
            &symbol.kind,
            &pending.file.rel_path,
            symbol.line,
            symbol.col,
            symbol.byte_start,
            symbol.byte_end,
        )?;
    }

    for call in &extracts.calls {
        db.insert_symbol(
            &call.callee_name,
            "call",
            &pending.file.rel_path,
            call.line,
            call.col,
            0,
            0,
        )?;
    }

    for render in &extracts.renders {
        db.insert_symbol(
            &render.component_name,
            "render",
            &pending.file.rel_path,
            render.line,
            render.col,
            0,
            0,
        )?;
    }

    for import in &extracts.imports {
        db.insert_symbol(
            &import.imported_name,
            "import",
            &pending.file.rel_path,
            import.line,
            0,
            0,
            0,
        )?;
        db.insert_import_ref(
            &pending.file.rel_path,
            &import.imported_name,
            &import.source_module,
            import.line,
        )?;
    }

    for route in &extracts.routes {
        let route_name = format!("{} {}", route.method, route.path);
        let route_id = db.insert_symbol(
            &route_name,
            "route",
            &pending.file.rel_path,
            route.line,
            route.col,
            0,
            0,
        )?;
        db.insert_route_ref(
            route_id,
            &route.handler_name,
            &pending.file.rel_path,
            route.line,
        )?;
    }

    db.upsert_file(
        &pending.file.rel_path,
        &pending.hash,
        pending.file.lang.name(),
        pending.file.last_modified,
    )?;
    Ok(true)
}

fn chrono_now() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}
