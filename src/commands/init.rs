use std::time::Instant;

use anyhow::{Context, Result};

use crate::db::Db;
use crate::extractor::ExtractorPool;
use crate::hasher;
use crate::resolver;
use crate::scan;
use crate::ui;

pub fn run(quiet: bool) -> Result<()> {
    let start = Instant::now();
    let cwd = std::env::current_dir().context("failed to get current directory")?;
    let link_dir = cwd.join(".link");

    let db = Db::open(&link_dir)?;
    let files = scan::collect_source_files(&cwd, quiet)?;

    if files.is_empty() {
        db.with_transaction(|db| {
            db.reset_index()?;
            db.write_index_metadata()?;
            db.set_meta("last_scan", &chrono_now())?;
            Ok(())
        })?;
        if !quiet {
            println!("No supported files found.");
        }
        return Ok(());
    }
    let mut extractor = ExtractorPool::default();
    let (indexed_files, total_symbols, resolve_stats) = db.with_transaction(|db| {
        db.reset_index()?;

        let mut indexed_files = 0usize;
        let mut total_symbols = 0u64;

        for file in &files {
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

            if std::str::from_utf8(&source).is_err() {
                if !quiet {
                    ui::warn(quiet, format!("skipping non-UTF8 file {}", file.rel_path));
                }
                continue;
            }

            let mut extracts = match extractor.extract(&source, file.lang) {
                Ok(extracts) => extracts,
                Err(err) => {
                    if !quiet {
                        ui::warn(
                            quiet,
                            format!("failed to parse file {}: {err}", file.rel_path),
                        );
                    }
                    continue;
                }
            };
            if matches!(file.lang, crate::lang::Lang::Php) && file.rel_path.starts_with("routes/") {
                if let Ok(text) = std::str::from_utf8(&source) {
                    extracts.routes.extend(
                        crate::framework::laravel::extract_routes_from_routes_php(text),
                    );
                }
            }

            let hash = hasher::hash_bytes(&source);
            insert_extracts(db, file, &extracts, &hash)?;
            indexed_files += 1;
            total_symbols += (extracts.symbols.len()
                + extracts.calls.len()
                + extracts.renders.len()
                + extracts.imports.len()
                + extracts.routes.len()) as u64;
        }

        db.clear_edges()?;
        let resolve_stats = resolver::resolve(db)?;
        db.write_index_metadata()?;
        db.set_meta("last_scan", &chrono_now())?;
        Ok((indexed_files, total_symbols, resolve_stats))
    })?;

    let elapsed = start.elapsed();
    let edge_count = db.edge_count()?;

    if !quiet {
        ui::info(
            quiet,
            format!(
            "Initialized .link/index.db\n  {} files | {} symbols | {} edges | {:.1}s\n  ({} calls resolved, {} renders resolved, {} ambiguous, {} imports linked)",
            indexed_files,
            total_symbols,
            edge_count,
            elapsed.as_secs_f64(),
            resolve_stats.resolved,
            resolve_stats.renders,
            resolve_stats.ambiguous,
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

fn insert_extracts(
    db: &Db,
    file: &scan::SourceFile,
    extracts: &crate::extractor::FileExtracts,
    hash: &str,
) -> Result<()> {
    for symbol in &extracts.symbols {
        db.insert_symbol(
            &symbol.name,
            &symbol.kind,
            &file.rel_path,
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
            &file.rel_path,
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
            &file.rel_path,
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
            &file.rel_path,
            import.line,
            0,
            0,
            0,
        )?;
        db.insert_import_ref(
            &file.rel_path,
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
            &file.rel_path,
            route.line,
            route.col,
            0,
            0,
        )?;
        db.insert_route_ref(route_id, &route.handler_name, &file.rel_path, route.line)?;
    }

    db.upsert_file(&file.rel_path, hash, file.lang.name(), file.last_modified)?;
    Ok(())
}

fn chrono_now() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    now.to_string()
}
