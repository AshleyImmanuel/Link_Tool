use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::db::Db;
use crate::db::ImportRef;

/// Resolve cross-file call edges using a simple 3-tier strategy:
///
/// 1. Same-file: call name matches a definition in the same file -> edge
/// 2. Import-based: call name matches an imported name, and an export exists -> edge
/// 3. Global fallback: exactly one top-level definition matches anywhere -> edge
///
/// Ambiguous matches (multiple candidates) are skipped to avoid false positives.
pub fn resolve(db: &Db) -> Result<ResolveStats> {
    let definitions = db.definition_symbols()?;
    let call_symbols = db.symbols_by_kind("call")?;
    let render_symbols = db.symbols_by_kind("render")?;
    let import_symbols = db.symbols_by_kind("import")?;
    let route_symbols = db.symbols_by_kind("route")?;
    let mut stats = ResolveStats::default();

    let mut defs_by_name: HashMap<String, Vec<(i64, String, String)>> = HashMap::new();
    for definition in &definitions {
        defs_by_name
            .entry(definition.name.clone())
            .or_default()
            .push((
                definition.id,
                definition.file.clone(),
                definition.kind.clone(),
            ));
    }

    for caller in &call_symbols {
        let candidates = match defs_by_name.get(&caller.name) {
            Some(candidates) => candidates,
            None => continue,
        };

        let same_file: Vec<_> = candidates
            .iter()
            .filter(|(_, file, _)| file == &caller.file)
            .collect();

        if same_file.len() == 1 {
            db.insert_edge(
                caller.id,
                same_file[0].0,
                "calls",
                "same-file definition match",
                &caller.file,
                caller.line,
                0.95,
            )?;
            stats.resolved += 1;
            continue;
        }

        if let Some((target_id, reason, confidence)) =
            resolve_via_imports(db, &caller.file, &caller.name, candidates)?
        {
            db.insert_edge(
                caller.id,
                target_id,
                "calls",
                &reason,
                &caller.file,
                caller.line,
                confidence,
            )?;
            stats.resolved += 1;
            continue;
        }

        if candidates.len() == 1 {
            db.insert_edge(
                caller.id,
                candidates[0].0,
                "calls",
                "unique global definition match",
                &caller.file,
                caller.line,
                0.70,
            )?;
            stats.resolved += 1;
            continue;
        }

        stats.ambiguous += 1;
    }

    for render in &render_symbols {
        let candidates = match defs_by_name.get(&render.name) {
            Some(candidates) => candidates,
            None => continue,
        };

        let same_file: Vec<_> = candidates
            .iter()
            .filter(|(_, file, _)| file == &render.file)
            .collect();

        if same_file.len() == 1 {
            db.insert_edge(
                render.id,
                same_file[0].0,
                "renders",
                "same-file JSX component render",
                &render.file,
                render.line,
                0.97,
            )?;
            stats.renders += 1;
            continue;
        }

        if candidates.len() == 1 {
            db.insert_edge(
                render.id,
                candidates[0].0,
                "renders",
                "unique JSX component match",
                &render.file,
                render.line,
                0.90,
            )?;
            stats.renders += 1;
        }
    }

    for import in &import_symbols {
        if let Some(candidates) = defs_by_name.get(&import.name) {
            let other_file: Vec<_> = candidates
                .iter()
                .filter(|(_, file, _)| file != &import.file)
                .collect();
            if other_file.len() == 1 {
                db.insert_edge(
                    import.id,
                    other_file[0].0,
                    "imports",
                    "import name match across files",
                    &import.file,
                    import.line,
                    0.99,
                )?;
                stats.imports += 1;
            }
        }
    }

    // Route -> handler edges (Express/Laravel heuristics)
    let route_refs = db.all_route_refs()?;
    let mut route_by_id: HashMap<i64, (String, u32)> = HashMap::new();
    for route in &route_symbols {
        route_by_id.insert(route.id, (route.file.clone(), route.line));
    }
    for route_ref in route_refs {
        let Some((route_file, route_line)) = route_by_id.get(&route_ref.route_id) else {
            continue;
        };
        let candidates = match defs_by_name.get(&route_ref.handler_name) {
            Some(c) => Some(c),
            None => {
                // Laravel-style "Controller@method" fallback: try method, then controller class.
                if let Some((controller, method)) = route_ref.handler_name.split_once('@') {
                    defs_by_name
                        .get(method)
                        .or_else(|| defs_by_name.get(controller))
                } else {
                    None
                }
            }
        };
        let Some(candidates) = candidates else {
            continue;
        };
        let same_file: Vec<_> = candidates
            .iter()
            .filter(|(_, file, _)| file == route_file)
            .collect();
        if same_file.len() == 1 {
            db.insert_edge(
                route_ref.route_id,
                same_file[0].0,
                "routes_to",
                "route handler match (same-file)",
                &route_ref.origin_file,
                route_ref.origin_line,
                0.96,
            )?;
            stats.routes += 1;
            continue;
        }
        if candidates.len() == 1 {
            db.insert_edge(
                route_ref.route_id,
                candidates[0].0,
                "routes_to",
                "route handler match (unique global)",
                &route_ref.origin_file,
                route_ref.origin_line,
                0.78,
            )?;
            stats.routes += 1;
            continue;
        }
        stats.ambiguous += 1;
        let _ = (route_line, route_file);
    }

    Ok(stats)
}

#[derive(Debug, Default)]
pub struct ResolveStats {
    pub resolved: usize,
    pub ambiguous: usize,
    pub imports: usize,
    pub renders: usize,
    pub routes: usize,
}

fn resolve_via_imports(
    db: &Db,
    caller_file: &str,
    call_name: &str,
    candidates: &[(i64, String, String)],
) -> Result<Option<(i64, String, f32)>> {
    let imports = db.import_refs_for_file(caller_file)?;
    let matching: Vec<ImportRef> = imports
        .into_iter()
        .filter(|imp| imp.imported_name == call_name)
        .collect();
    if matching.is_empty() {
        return Ok(None);
    }

    // Heuristic 1: if we can resolve a relative import to a concrete file path, prefer that.
    let caller_dir = Path::new(caller_file).parent().unwrap_or(Path::new(""));
    for imp in &matching {
        if let Some(resolved_paths) = resolve_relative_module_paths(caller_dir, &imp.source_module)
        {
            let path_matches: Vec<_> = candidates
                .iter()
                .filter(|(_, file, _)| resolved_paths.iter().any(|p| p == file))
                .collect();
            if path_matches.len() == 1 {
                return Ok(Some((
                    path_matches[0].0,
                    format!("import-based match via {}", imp.source_module),
                    0.88,
                )));
            }
        }
    }

    // Heuristic 2: the name is imported here, and there is exactly one candidate definition
    // in other files. This is weaker, but still better than global-only fallback.
    let other_file: Vec<_> = candidates
        .iter()
        .filter(|(_, file, _)| file != caller_file)
        .collect();
    if other_file.len() == 1 {
        let source = matching
            .first()
            .map(|imp| imp.source_module.as_str())
            .unwrap_or("<import>");
        return Ok(Some((
            other_file[0].0,
            format!("imported name match (source: {})", source),
            0.75,
        )));
    }

    Ok(None)
}

fn resolve_relative_module_paths(caller_dir: &Path, source_module: &str) -> Option<Vec<String>> {
    let raw = source_module.trim();
    if raw.is_empty() || !raw.starts_with('.') {
        return None;
    }

    // Normalize to forward slashes to match what Link stores in DB.
    let raw = raw.replace('\\', "/");
    let base = caller_dir.join(raw);

    let mut candidates = Vec::new();
    let base_norm = normalize_rel_path(&base);
    candidates.push(base_norm.clone());

    // If module already has an extension, also accept it as-is.
    if has_known_ext(&base_norm) {
        return Some(dedupe(candidates));
    }

    for ext in [".ts", ".tsx", ".js", ".jsx", ".py", ".go", ".rs"] {
        candidates.push(format!("{base_norm}{ext}"));
    }

    for ext in [".ts", ".tsx", ".js", ".jsx"] {
        candidates.push(format!("{base_norm}/index{ext}"));
    }

    Some(dedupe(candidates))
}

fn normalize_rel_path(path: &Path) -> String {
    // Best-effort normalization for relative paths:
    // - convert separators to '/'
    // - collapse '.' segments
    // - resolve '..' segments without going above root
    let mut parts: Vec<String> = Vec::new();
    for component in path.components() {
        use std::path::Component;
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                let _ = parts.pop();
            }
            Component::Normal(seg) => parts.push(seg.to_string_lossy().to_string()),
            // For safety, ignore any absolute/prefix components; Link stores rel paths.
            Component::RootDir | Component::Prefix(_) => {}
        }
    }
    parts.join("/")
}

fn has_known_ext(path: &str) -> bool {
    matches!(
        PathBuf::from(path).extension().and_then(|e| e.to_str()),
        Some("ts" | "tsx" | "js" | "jsx" | "py" | "go" | "rs")
    )
}

fn dedupe(items: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashMap::new();
    let mut out = Vec::new();
    for item in items {
        if seen.insert(item.clone(), ()).is_none() {
            out.push(item);
        }
    }
    out
}
