use anyhow::{Context, Result};

use crate::db::{Db, Edge, Symbol};
use crate::error::user_error;
use crate::intel;
use crate::viewer;

pub fn run(query: &str) -> Result<()> {
    let cwd = std::env::current_dir().context("failed to get current directory")?;
    let link_dir = cwd.join(".link");

    if !link_dir.join("index.db").exists() {
        return Err(user_error("not a Link project. Run 'linkmap init' first."));
    }

    let db = Db::open_index(&link_dir)?;

    if let Some((from, to)) = intel::parse_path_query(query) {
        return explain_path(&db, &from, &to);
    }

    let symbols = db.find_symbols_by_name(query)?;
    let defs: Vec<_> = symbols
        .iter()
        .filter(|symbol| intel::is_definition_kind(&symbol.kind))
        .collect();

    if defs.is_empty() {
        let fuzzy = db.fuzzy_search(query)?;
        let fuzzy_defs: Vec<_> = fuzzy
            .iter()
            .filter(|symbol| intel::is_definition_kind(&symbol.kind))
            .take(5)
            .collect();

        if fuzzy_defs.is_empty() {
            return Err(user_error(format!("symbol '{}' not found.", query)));
        }

        println!("No exact match for '{}'. Did you mean:", query);
        for symbol in &fuzzy_defs {
            println!(
                "  {} ({}) {}:{}",
                symbol.name, symbol.kind, symbol.file, symbol.line
            );
        }
        return Ok(());
    }

    let changed_files = intel::changed_files(&cwd)?.unwrap_or_default();
    let violations = intel::architecture_violations(&cwd, &db)?;

    for target in &defs {
        explain_symbol(&db, target, &cwd, &changed_files, &violations)?;
    }

    Ok(())
}

fn explain_path(db: &Db, from: &str, to: &str) -> Result<()> {
    let Some(path) = intel::find_path(db, from, to)? else {
        return Err(user_error(format!(
            "no path found from '{}' to '{}'.",
            from, to
        )));
    };

    println!("Path: {} -> {}", path.from_query, path.to_query);
    println!();

    if path.steps.is_empty() {
        println!("  Start and end resolved to the same symbol.");
        return Ok(());
    }

    for (index, step) in path.steps.iter().enumerate() {
        println!(
            "  {}. {} [{}] {}:{}",
            index + 1,
            step.from.name,
            intel::semantic_kind(&step.from),
            step.from.file,
            step.from.line
        );
        println!(
            "     --{}--> {} [{}] {}:{}",
            step.edge.relation,
            step.to.name,
            intel::semantic_kind(&step.to),
            step.to.file,
            step.to.line
        );
        println!(
            "     why: {} | confidence: {}% | origin: {}:{}",
            step.edge.reason,
            (step.edge.confidence * 100.0).round(),
            step.edge.origin_file,
            step.edge.origin_line
        );
    }

    Ok(())
}

fn explain_symbol(
    db: &Db,
    target: &Symbol,
    root: &std::path::Path,
    changed_files: &std::collections::HashSet<String>,
    violations: &[intel::RuleViolation],
) -> Result<()> {
    println!(
        "{} ({}, {}) - {}:{}",
        target.name,
        intel::semantic_kind(target),
        target.kind,
        target.file,
        target.line
    );
    if changed_files.contains(&target.file) {
        println!("  Changed in local git working tree (vs HEAD)");
    }
    println!();

    let edges_and_symbols = db.edges_for_symbol(target.id)?;
    let mut called_by = Vec::new();
    let mut calls = Vec::new();
    let mut imported_by = Vec::new();
    let mut rendered_by = Vec::new();

    for item in &edges_and_symbols {
        match item.0.relation.as_str() {
            "calls" if item.0.from_id == target.id => calls.push(item),
            "calls" if item.0.to_id == target.id => called_by.push(item),
            "imports" if item.0.to_id == target.id => imported_by.push(item),
            "renders" if item.0.to_id == target.id => rendered_by.push(item),
            _ => {}
        }
    }

    print_relation_group("Rendered by", &rendered_by, false);
    print_relation_group("Imported by", &imported_by, false);
    print_relation_group("Called by", &called_by, false);
    print_relation_group("Calls", &calls, true);

    let graph = viewer::build_graph(db, target, root)?;
    let impacted: Vec<_> = graph
        .nodes
        .iter()
        .filter(|node| !node.is_center && node.impact_depth > 0)
        .take(8)
        .collect();

    if !impacted.is_empty() {
        println!("  Likely impact:");
        for node in impacted {
            println!(
                "    {} [{}] {}:{}",
                node.label, node.kind, node.file, node.line
            );
        }
        println!();
    }

    let file_violations: Vec<_> = violations
        .iter()
        .filter(|violation| violation.file == target.file)
        .collect();
    if !file_violations.is_empty() {
        println!("  Rule violations (heuristic):");
        for violation in file_violations {
            println!(
                "    {}:{} [{}] {} -> {}",
                violation.file,
                violation.line,
                violation.rule,
                violation.import_target,
                violation.detail
            );
        }
        println!();
    }

    if called_by.is_empty() && calls.is_empty() && imported_by.is_empty() && rendered_by.is_empty()
    {
        println!("  No direct connections found.");
        println!();
    }

    Ok(())
}

fn print_relation_group(title: &str, items: &[&(Edge, Symbol)], outgoing: bool) {
    if items.is_empty() {
        return;
    }

    println!("  {}:", title);
    for (edge, symbol) in items {
        let location = format!("{}:{}", symbol.file, symbol.line);
        let direction = if outgoing { "->" } else { "<-" };
        println!(
            "    {} {} {} ({})",
            direction,
            symbol.name,
            location,
            intel::semantic_kind(symbol)
        );
        println!(
            "      why: {} | confidence: {}% | origin: {}:{}",
            edge.reason,
            (edge.confidence * 100.0).round(),
            edge.origin_file,
            edge.origin_line
        );
    }
    println!();
}
