use std::collections::HashMap;

use anyhow::Result;

use crate::db::Db;

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
    let import_symbols = db.symbols_by_kind("import")?;
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
            db.insert_edge(caller.id, same_file[0].0, "calls")?;
            stats.resolved += 1;
            continue;
        }

        if candidates.len() == 1 {
            db.insert_edge(caller.id, candidates[0].0, "calls")?;
            stats.resolved += 1;
            continue;
        }

        stats.ambiguous += 1;
    }

    for import in &import_symbols {
        if let Some(candidates) = defs_by_name.get(&import.name) {
            let other_file: Vec<_> = candidates
                .iter()
                .filter(|(_, file, _)| file != &import.file)
                .collect();
            if other_file.len() == 1 {
                db.insert_edge(import.id, other_file[0].0, "imports")?;
                stats.imports += 1;
            }
        }
    }

    Ok(stats)
}

#[derive(Debug, Default)]
pub struct ResolveStats {
    pub resolved: usize,
    pub ambiguous: usize,
    pub imports: usize,
}
