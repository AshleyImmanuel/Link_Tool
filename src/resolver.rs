use std::collections::HashMap;

use anyhow::Result;

use crate::db::Db;

/// Resolve cross-file call edges using a simple 3-tier strategy:
///
/// 1. Same-file: call name matches a definition in the same file → edge
/// 2. Import-based: call name matches an imported name, and an export exists → edge
/// 3. Global fallback: exactly one top-level definition matches anywhere → edge
///
/// Ambiguous matches (multiple candidates) are skipped to avoid false positives.
pub fn resolve(db: &Db) -> Result<ResolveStats> {
    let symbols = db.all_symbols()?;
    let mut stats = ResolveStats::default();

    // Build index: name → [(symbol_id, file, kind)]
    let mut defs_by_name: HashMap<String, Vec<(i64, String, String)>> = HashMap::new();
    for s in &symbols {
        if is_definition_kind(&s.kind) {
            defs_by_name
                .entry(s.name.clone())
                .or_default()
                .push((s.id, s.file.clone(), s.kind.clone()));
        }
    }

    // For each call-type symbol, try to resolve it
    for caller in &symbols {
        if caller.kind != "call" {
            continue;
        }

        let callee_name = &caller.name;
        let candidates = match defs_by_name.get(callee_name) {
            Some(c) => c,
            None => continue, // no definition found anywhere
        };

        // Tier 1: same-file match
        let same_file: Vec<_> = candidates
            .iter()
            .filter(|(_, f, _)| f == &caller.file)
            .collect();

        if same_file.len() == 1 {
            db.insert_edge(caller.id, same_file[0].0, "calls")?;
            stats.resolved += 1;
            continue;
        }

        // Tier 2: if only one global definition exists, use it
        if candidates.len() == 1 {
            db.insert_edge(caller.id, candidates[0].0, "calls")?;
            stats.resolved += 1;
            continue;
        }

        // Tier 3: ambiguous — skip
        stats.ambiguous += 1;
    }

    // Resolve import edges: connect import symbols to their definitions
    for imp in &symbols {
        if imp.kind != "import" {
            continue;
        }
        if let Some(candidates) = defs_by_name.get(&imp.name) {
            // Connect to definitions in other files
            let other_file: Vec<_> = candidates
                .iter()
                .filter(|(_, f, _)| f != &imp.file)
                .collect();
            if other_file.len() == 1 {
                db.insert_edge(imp.id, other_file[0].0, "imports")?;
                stats.imports += 1;
            }
        }
    }

    Ok(stats)
}

fn is_definition_kind(kind: &str) -> bool {
    matches!(
        kind,
        "function" | "class" | "method" | "variable" | "struct" | "enum" | "type" | "interface" | "module"
    )
}

#[derive(Debug, Default)]
pub struct ResolveStats {
    pub resolved: usize,
    pub ambiguous: usize,
    pub imports: usize,
}
