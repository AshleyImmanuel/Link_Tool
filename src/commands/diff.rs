use std::path::Path;

use anyhow::Result;

use crate::error::user_error;
use crate::snapshot;

pub fn run(from: &str, to: &str, json: bool) -> Result<()> {
    let from_path = Path::new(from);
    let to_path = Path::new(to);

    if !from_path.exists() {
        return Err(user_error(format!(
            "snapshot not found: {}",
            from_path.display()
        )));
    }
    if !to_path.exists() {
        return Err(user_error(format!(
            "snapshot not found: {}",
            to_path.display()
        )));
    }

    let from_snap = snapshot::read_snapshot(from_path)?;
    let to_snap = snapshot::read_snapshot(to_path)?;
    let diff = snapshot::diff_snapshots(&from_snap, &to_snap, from, to);

    if json {
        println!("{}", serde_json::to_string_pretty(&diff)?);
        return Ok(());
    }

    println!("Link Snapshot Diff");
    println!("  From: {}", diff.from);
    println!("  To:   {}", diff.to);
    println!();

    print_group("Added symbols", &diff.added_symbols, |s| {
        format!("+ {} ({}) {}:{}", s.name, s.kind, s.file, s.line)
    });
    print_group("Removed symbols", &diff.removed_symbols, |s| {
        format!("- {} ({}) {}:{}", s.name, s.kind, s.file, s.line)
    });
    print_group("Added edges", &diff.added_edges, |e| {
        format!(
            "+ {} --{}--> {}  (origin {}:{} | {}%)",
            edge_end_label(&e.from),
            e.relation,
            edge_end_label(&e.to),
            e.origin_file,
            e.origin_line,
            e.confidence
        )
    });
    print_group("Removed edges", &diff.removed_edges, |e| {
        format!(
            "- {} --{}--> {}  (origin {}:{} | {}%)",
            edge_end_label(&e.from),
            e.relation,
            edge_end_label(&e.to),
            e.origin_file,
            e.origin_line,
            e.confidence
        )
    });
    print_group("Added imports", &diff.added_imports, |i| {
        format!(
            "+ {}:{} imports {} from {}",
            i.file, i.line, i.imported_name, i.source_module
        )
    });
    print_group("Removed imports", &diff.removed_imports, |i| {
        format!(
            "- {}:{} imports {} from {}",
            i.file, i.line, i.imported_name, i.source_module
        )
    });

    Ok(())
}

fn edge_end_label(s: &snapshot::SnapshotSymbol) -> String {
    format!("{} ({})", s.name, short_path(&s.file))
}

fn short_path(path: &str) -> &str {
    path.rsplit_once('/').map(|(_, file)| file).unwrap_or(path)
}

fn print_group<T>(title: &str, items: &[T], render: impl Fn(&T) -> String) {
    if items.is_empty() {
        return;
    }

    const MAX: usize = 25;
    println!("{}:", title);
    for item in items.iter().take(MAX) {
        println!("  {}", render(item));
    }
    if items.len() > MAX {
        println!("  ... and {} more", items.len() - MAX);
    }
    println!();
}
