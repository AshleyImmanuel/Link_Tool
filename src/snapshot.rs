use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::db::{Db, Edge, ImportRef, Symbol};

pub const SNAPSHOT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub version: u32,
    pub created_at: u64,
    pub repo_root: Option<String>,
    pub symbols: Vec<SnapshotSymbol>,
    pub edges: Vec<SnapshotEdge>,
    pub import_refs: Vec<SnapshotImportRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Ord, PartialOrd)]
pub struct SnapshotSymbol {
    pub name: String,
    pub kind: String,
    pub file: String,
    pub line: u32,
    pub col: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Ord, PartialOrd)]
pub struct SnapshotEdge {
    pub from: SnapshotSymbol,
    pub to: SnapshotSymbol,
    pub relation: String,
    pub reason: String,
    pub origin_file: String,
    pub origin_line: u32,
    pub confidence: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Ord, PartialOrd)]
pub struct SnapshotImportRef {
    pub file: String,
    pub imported_name: String,
    pub source_module: String,
    pub line: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotDiff {
    pub version: u32,
    pub from: String,
    pub to: String,
    pub added_symbols: Vec<SnapshotSymbol>,
    pub removed_symbols: Vec<SnapshotSymbol>,
    pub added_edges: Vec<SnapshotEdge>,
    pub removed_edges: Vec<SnapshotEdge>,
    pub added_imports: Vec<SnapshotImportRef>,
    pub removed_imports: Vec<SnapshotImportRef>,
}

pub fn build_snapshot(db: &Db, repo_root: Option<&Path>) -> Result<Snapshot> {
    let all_symbols = db.list_all_symbols()?;
    let all_edges = db.all_edges()?;
    let all_imports = db.all_import_refs()?;

    let mut by_id = std::collections::HashMap::new();
    for s in all_symbols {
        by_id.insert(s.id, s);
    }

    let mut symbols = Vec::new();
    for symbol in by_id.values() {
        symbols.push(symbol_to_snapshot(symbol));
    }
    symbols.sort();
    symbols.dedup();

    let mut edges = Vec::new();
    for edge in all_edges {
        let Some(from) = by_id.get(&edge.from_id) else {
            continue;
        };
        let Some(to) = by_id.get(&edge.to_id) else {
            continue;
        };
        edges.push(edge_to_snapshot(&edge, from, to));
    }
    edges.sort();
    edges.dedup();

    let mut import_refs = all_imports
        .into_iter()
        .map(import_ref_to_snapshot)
        .collect::<Vec<_>>();
    import_refs.sort();
    import_refs.dedup();

    Ok(Snapshot {
        version: SNAPSHOT_VERSION,
        created_at: now_epoch(),
        repo_root: repo_root.map(|p| p.to_string_lossy().to_string()),
        symbols,
        edges,
        import_refs,
    })
}

pub fn write_snapshot(path: &Path, snapshot: &Snapshot) -> Result<()> {
    let json = serde_json::to_string_pretty(snapshot).context("failed to serialize snapshot")?;
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
    }
    std::fs::write(path, json).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

pub fn read_snapshot(path: &Path) -> Result<Snapshot> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let snapshot: Snapshot = serde_json::from_str(&raw).context("failed to parse snapshot JSON")?;
    Ok(snapshot)
}

pub fn diff_snapshots(
    from: &Snapshot,
    to: &Snapshot,
    from_label: &str,
    to_label: &str,
) -> SnapshotDiff {
    use std::collections::BTreeSet;

    let from_symbols: BTreeSet<_> = from.symbols.iter().cloned().collect();
    let to_symbols: BTreeSet<_> = to.symbols.iter().cloned().collect();

    let from_edges: BTreeSet<_> = from.edges.iter().cloned().collect();
    let to_edges: BTreeSet<_> = to.edges.iter().cloned().collect();

    let from_imports: BTreeSet<_> = from.import_refs.iter().cloned().collect();
    let to_imports: BTreeSet<_> = to.import_refs.iter().cloned().collect();

    let mut added_symbols = to_symbols
        .difference(&from_symbols)
        .cloned()
        .collect::<Vec<_>>();
    let mut removed_symbols = from_symbols
        .difference(&to_symbols)
        .cloned()
        .collect::<Vec<_>>();
    let mut added_edges = to_edges
        .difference(&from_edges)
        .cloned()
        .collect::<Vec<_>>();
    let mut removed_edges = from_edges
        .difference(&to_edges)
        .cloned()
        .collect::<Vec<_>>();
    let mut added_imports = to_imports
        .difference(&from_imports)
        .cloned()
        .collect::<Vec<_>>();
    let mut removed_imports = from_imports
        .difference(&to_imports)
        .cloned()
        .collect::<Vec<_>>();

    added_symbols.sort();
    removed_symbols.sort();
    added_edges.sort();
    removed_edges.sort();
    added_imports.sort();
    removed_imports.sort();

    SnapshotDiff {
        version: SNAPSHOT_VERSION,
        from: from_label.to_string(),
        to: to_label.to_string(),
        added_symbols,
        removed_symbols,
        added_edges,
        removed_edges,
        added_imports,
        removed_imports,
    }
}

pub fn default_snapshot_path(link_dir: &Path) -> PathBuf {
    link_dir.join("snapshot.json")
}

fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn symbol_to_snapshot(symbol: &Symbol) -> SnapshotSymbol {
    SnapshotSymbol {
        name: symbol.name.clone(),
        kind: symbol.kind.clone(),
        file: normalize_path(&symbol.file),
        line: symbol.line,
        col: symbol.col,
    }
}

fn edge_to_snapshot(edge: &Edge, from: &Symbol, to: &Symbol) -> SnapshotEdge {
    SnapshotEdge {
        from: symbol_to_snapshot(from),
        to: symbol_to_snapshot(to),
        relation: edge.relation.clone(),
        reason: edge.reason.clone(),
        origin_file: normalize_path(&edge.origin_file),
        origin_line: edge.origin_line,
        confidence: (edge.confidence * 100.0).round().clamp(0.0, 100.0) as u32,
    }
}

fn import_ref_to_snapshot(imp: ImportRef) -> SnapshotImportRef {
    SnapshotImportRef {
        file: normalize_path(&imp.file),
        imported_name: imp.imported_name,
        source_module: imp.source_module,
        line: imp.line,
    }
}

fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
}
