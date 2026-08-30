use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};

use crate::db::{Db, Edge, Symbol};
use crate::extractor::{ExtractorPool, FileExtracts};
use crate::lang::{detect_lang, Lang};

const MAX_CHANGE_ITEMS: usize = 12;

#[derive(Debug, Clone)]
pub struct RuleViolation {
    pub rule: &'static str,
    pub file: String,
    pub line: u32,
    pub import_target: String,
    pub detail: String,
}

#[derive(Debug, Clone)]
pub struct PathStep {
    pub edge: Edge,
    pub from: Symbol,
    pub to: Symbol,
}

#[derive(Debug, Clone)]
pub struct PathResult {
    pub from_query: String,
    pub to_query: String,
    pub steps: Vec<PathStep>,
}

#[derive(Debug, Clone, Default)]
pub struct ChangeSummary {
    pub changed_files: Vec<String>,
    pub added_symbols: Vec<String>,
    pub removed_symbols: Vec<String>,
    pub added_edges: Vec<String>,
    pub removed_edges: Vec<String>,
}

pub fn is_definition_kind(kind: &str) -> bool {
    !matches!(kind, "call" | "import" | "render")
}

pub fn semantic_kind(symbol: &Symbol) -> &'static str {
    if is_route_file(&symbol.file) {
        "route"
    } else if is_handler_file(&symbol.file) {
        "handler"
    } else if is_component_symbol(symbol) {
        "component"
    } else {
        match symbol.kind.as_str() {
            "render" => "render",
            "call" => "call",
            "import" => "import",
            "class" | "struct" | "enum" | "type" | "interface" => "class",
            "function" => "function",
            "method" => "method",
            "variable" => "variable",
            _ => "other",
        }
    }
}

pub fn semantic_label(symbol: &Symbol) -> String {
    let location = format!("{}:{}", short_path(&symbol.file), symbol.line);
    match semantic_kind(symbol) {
        "route" => {
            let route = route_path_for_file(&symbol.file).unwrap_or_else(|| symbol.file.clone());
            format!("{} ({})", route, location)
        }
        _ => format!("{} ({})", symbol.name, location),
    }
}

pub fn route_path_for_file(file: &str) -> Option<String> {
    let normalized = normalize_rel_path(file);
    if !normalized.starts_with("app/") {
        return None;
    }

    if !matches!(
        normalized.rsplit('/').next(),
        Some(
            "page.tsx"
                | "page.ts"
                | "page.jsx"
                | "page.js"
                | "layout.tsx"
                | "layout.ts"
                | "layout.jsx"
                | "layout.js"
        )
    ) {
        return None;
    }

    let mut segments = Vec::new();
    for segment in normalized.split('/').skip(1) {
        if matches!(
            segment,
            "page.tsx"
                | "page.ts"
                | "page.jsx"
                | "page.js"
                | "layout.tsx"
                | "layout.ts"
                | "layout.jsx"
                | "layout.js"
        ) {
            continue;
        }
        if segment.starts_with('(') && segment.ends_with(')') {
            continue;
        }
        segments.push(segment);
    }

    if segments.is_empty() {
        Some("/".to_string())
    } else {
        Some(format!("/{}", segments.join("/")))
    }
}

pub fn changed_files(root: &Path) -> Result<Option<HashSet<String>>> {
    let Some(raw_paths) = git_status_paths(root)? else {
        return Ok(None);
    };

    let changed = raw_paths
        .into_iter()
        .map(|path| normalize_rel_path(&path))
        .collect::<HashSet<_>>();
    Ok(Some(changed))
}

pub fn architecture_violations(root: &Path, db: &Db) -> Result<Vec<RuleViolation>> {
    let imports = db.all_import_refs()?;
    let mut client_cache = HashMap::new();
    let mut violations = Vec::new();

    for import in imports {
        let target = import.source_module.trim();
        if target.is_empty() {
            continue;
        }

        if is_ui_file(&import.file) && looks_like_data_layer(target) {
            violations.push(RuleViolation {
                rule: "ui-no-db",
                file: import.file.clone(),
                line: import.line,
                import_target: format!("{} from {}", import.imported_name, target),
                detail: "UI/component code should not depend on data-layer modules".to_string(),
            });
        }

        if is_route_file(&import.file) && looks_like_api_module(target) {
            violations.push(RuleViolation {
                rule: "route-no-api-import",
                file: import.file.clone(),
                line: import.line,
                import_target: format!("{} from {}", import.imported_name, target),
                detail: "Route/page files should not import API handler modules".to_string(),
            });
        }

        if is_client_file(root, &import.file, &mut client_cache)? && looks_like_server_only(target)
        {
            violations.push(RuleViolation {
                rule: "client-no-server",
                file: import.file.clone(),
                line: import.line,
                import_target: format!("{} from {}", import.imported_name, target),
                detail: "Client components should not import server-only modules".to_string(),
            });
        }
    }

    Ok(violations)
}

pub fn find_path(db: &Db, from_query: &str, to_query: &str) -> Result<Option<PathResult>> {
    let all_symbols = db.list_all_symbols()?;
    let mut symbol_by_id = HashMap::new();
    for symbol in all_symbols {
        symbol_by_id.insert(symbol.id, symbol);
    }

    let start_symbols = resolve_endpoint_candidates(symbol_by_id.values(), from_query, true);
    let end_symbols = resolve_endpoint_candidates(symbol_by_id.values(), to_query, false);
    if start_symbols.is_empty() || end_symbols.is_empty() {
        return Ok(None);
    }

    let end_ids = end_symbols
        .iter()
        .map(|symbol| symbol.id)
        .collect::<HashSet<_>>();
    let all_edges = db.all_edges()?;
    let mut outgoing: HashMap<i64, Vec<Edge>> = HashMap::new();
    for edge in all_edges {
        outgoing.entry(edge.from_id).or_default().push(edge);
    }

    let mut queue = VecDeque::new();
    let mut previous: HashMap<i64, (i64, Edge)> = HashMap::new();
    let mut seen = HashSet::new();

    for symbol in &start_symbols {
        seen.insert(symbol.id);
        queue.push_back(symbol.id);
    }

    let mut destination = None;
    while let Some(current) = queue.pop_front() {
        if end_ids.contains(&current) {
            destination = Some(current);
            break;
        }

        for edge in outgoing.get(&current).into_iter().flatten() {
            if seen.insert(edge.to_id) {
                previous.insert(edge.to_id, (current, edge.clone()));
                queue.push_back(edge.to_id);
            }
        }
    }

    let Some(mut cursor) = destination else {
        return Ok(None);
    };

    let mut steps = Vec::new();
    while let Some((prev, edge)) = previous.get(&cursor).cloned() {
        let Some(from_symbol) = symbol_by_id.get(&prev).cloned() else {
            break;
        };
        let Some(to_symbol) = symbol_by_id.get(&cursor).cloned() else {
            break;
        };
        steps.push(PathStep {
            edge,
            from: from_symbol,
            to: to_symbol,
        });
        cursor = prev;
    }
    steps.reverse();

    Ok(Some(PathResult {
        from_query: from_query.trim().to_string(),
        to_query: to_query.trim().to_string(),
        steps,
    }))
}

pub fn parse_path_query(input: &str) -> Option<(String, String)> {
    let (from, to) = input.split_once("->")?;
    let from = from.trim();
    let to = to.trim();
    if from.is_empty() || to.is_empty() {
        return None;
    }
    Some((from.to_string(), to.to_string()))
}

pub fn collect_change_summary(root: &Path) -> Result<Option<ChangeSummary>> {
    let Some(paths) = git_status_paths(root)? else {
        return Ok(None);
    };

    let mut extractor = ExtractorPool::default();
    let mut summary = ChangeSummary::default();
    let mut added_symbols = Vec::new();
    let mut removed_symbols = Vec::new();
    let mut added_edges = Vec::new();
    let mut removed_edges = Vec::new();

    for rel_path in paths {
        let normalized = normalize_rel_path(&rel_path);
        summary.changed_files.push(normalized.clone());

        let abs_path = root.join(&normalized);
        let Some(lang) = detect_lang(Path::new(&normalized)) else {
            continue;
        };

        let current_extracts = if abs_path.is_file() {
            let source = std::fs::read(&abs_path)
                .with_context(|| format!("failed to read {}", abs_path.display()))?;
            parse_extracts(&mut extractor, &source, lang)?
        } else {
            FileExtracts::default()
        };

        let head_extracts = match git_show_file(root, &normalized)? {
            Some(source) => parse_extracts(&mut extractor, source.as_bytes(), lang)?,
            None => FileExtracts::default(),
        };

        diff_file_extracts(
            &normalized,
            &current_extracts,
            &head_extracts,
            &mut added_symbols,
            &mut removed_symbols,
            &mut added_edges,
            &mut removed_edges,
        );
    }

    summary.changed_files.sort();
    summary.changed_files.dedup();
    summary.added_symbols = take_limited(added_symbols);
    summary.removed_symbols = take_limited(removed_symbols);
    summary.added_edges = take_limited(added_edges);
    summary.removed_edges = take_limited(removed_edges);
    Ok(Some(summary))
}

fn resolve_endpoint_candidates<'a>(
    symbols: impl Iterator<Item = &'a Symbol>,
    query: &str,
    include_non_definitions: bool,
) -> Vec<Symbol> {
    let normalized_query = normalize_rel_path(query);
    let mut exact_defs = Vec::new();
    let mut file_matches = Vec::new();

    for symbol in symbols {
        if symbol.name == query && (include_non_definitions || is_definition_kind(&symbol.kind)) {
            exact_defs.push(symbol.clone());
            continue;
        }

        let file = normalize_rel_path(&symbol.file);
        let short = short_path(&file);
        let matches_file = file == normalized_query
            || file.ends_with(&normalized_query)
            || short == normalized_query
            || short == query;

        if matches_file && (include_non_definitions || is_definition_kind(&symbol.kind)) {
            file_matches.push(symbol.clone());
        }
    }

    if !exact_defs.is_empty() {
        exact_defs
    } else {
        file_matches
    }
}

fn parse_extracts(
    extractor: &mut ExtractorPool,
    source: &[u8],
    lang: Lang,
) -> Result<FileExtracts> {
    if std::str::from_utf8(source).is_err() {
        return Ok(FileExtracts::default());
    }

    match extractor.extract(source, lang) {
        Ok(extracts) => Ok(extracts),
        Err(_) => Ok(FileExtracts::default()),
    }
}

fn diff_file_extracts(
    file: &str,
    current: &FileExtracts,
    head: &FileExtracts,
    added_symbols: &mut Vec<String>,
    removed_symbols: &mut Vec<String>,
    added_edges: &mut Vec<String>,
    removed_edges: &mut Vec<String>,
) {
    let current_symbols = current
        .symbols
        .iter()
        .map(|symbol| {
            format!(
                "{}:{}:{}:{}",
                symbol.name, symbol.kind, symbol.line, symbol.col
            )
        })
        .collect::<HashSet<_>>();
    let head_symbols = head
        .symbols
        .iter()
        .map(|symbol| {
            format!(
                "{}:{}:{}:{}",
                symbol.name, symbol.kind, symbol.line, symbol.col
            )
        })
        .collect::<HashSet<_>>();

    for symbol in current_symbols.difference(&head_symbols) {
        added_symbols.push(format!("+ {} ({})", symbol, file));
    }
    for symbol in head_symbols.difference(&current_symbols) {
        removed_symbols.push(format!("- {} ({})", symbol, file));
    }

    let current_refs = extract_ref_signatures(current);
    let head_refs = extract_ref_signatures(head);

    for edge in current_refs.difference(&head_refs) {
        added_edges.push(format!("+ {} ({})", edge, file));
    }
    for edge in head_refs.difference(&current_refs) {
        removed_edges.push(format!("- {} ({})", edge, file));
    }
}

fn extract_ref_signatures(extracts: &FileExtracts) -> HashSet<String> {
    let mut refs = HashSet::new();
    for call in &extracts.calls {
        refs.insert(format!("calls:{}:{}", call.callee_name, call.line));
    }
    for render in &extracts.renders {
        refs.insert(format!("renders:{}:{}", render.component_name, render.line));
    }
    for import in &extracts.imports {
        refs.insert(format!(
            "imports:{}:{}:{}",
            import.imported_name, import.source_module, import.line
        ));
    }
    refs
}

fn take_limited(mut items: Vec<String>) -> Vec<String> {
    items.sort();
    items.dedup();
    items.truncate(MAX_CHANGE_ITEMS);
    items
}

fn git_status_paths(root: &Path) -> Result<Option<Vec<String>>> {
    if !is_git_repo(root)? {
        return Ok(None);
    }

    let output = Command::new("git")
        .current_dir(root)
        .args([
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=all",
            "--",
            ".",
        ])
        .output()
        .context("failed to run git status")?;
    if !output.status.success() {
        return Ok(None);
    }

    Ok(Some(parse_git_status_output(&output.stdout)))
}

fn git_show_file(root: &Path, rel_path: &str) -> Result<Option<String>> {
    if !is_git_repo(root)? {
        return Ok(None);
    }

    let output = Command::new("git")
        .current_dir(root)
        .args(["show", "--no-textconv", &format!("HEAD:{rel_path}")])
        .output()
        .context("failed to read file from git history")?;
    if !output.status.success() {
        return Ok(None);
    }

    Ok(Some(String::from_utf8_lossy(&output.stdout).into_owned()))
}

fn is_git_repo(root: &Path) -> Result<bool> {
    let output = Command::new("git")
        .current_dir(root)
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .context("failed to run git rev-parse")?;
    Ok(output.status.success() && String::from_utf8_lossy(&output.stdout).trim() == "true")
}

fn is_component_symbol(symbol: &Symbol) -> bool {
    matches!(symbol.kind.as_str(), "function" | "class" | "variable")
        && is_pascal_case(&symbol.name)
        && is_component_file(&symbol.file)
}

fn is_component_file(file: &str) -> bool {
    let file = normalize_rel_path(file);
    file.contains("/components/") || file.starts_with("components/")
}

fn is_ui_file(file: &str) -> bool {
    let file = normalize_rel_path(file);
    file.starts_with("components/") || file.contains("/components/")
}

fn is_route_file(file: &str) -> bool {
    let file = normalize_rel_path(file);
    file.starts_with("app/")
        && matches!(
            file.rsplit('/').next(),
            Some(
                "page.tsx"
                    | "page.ts"
                    | "page.jsx"
                    | "page.js"
                    | "layout.tsx"
                    | "layout.ts"
                    | "layout.jsx"
                    | "layout.js"
            )
        )
}

fn is_handler_file(file: &str) -> bool {
    let file = normalize_rel_path(file);
    file.starts_with("app/api/")
        || file.ends_with("/route.ts")
        || file.ends_with("/route.tsx")
        || file.ends_with("/route.js")
        || file.ends_with("/route.jsx")
        || file.starts_with("pages/api/")
}

fn is_pascal_case(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some(ch) if ch.is_ascii_uppercase()) && !name.contains('_')
}

fn looks_like_data_layer(import_target: &str) -> bool {
    let lower = import_target.to_ascii_lowercase();
    lower.contains("db")
        || lower.contains("prisma")
        || lower.contains("postgres")
        || lower.contains("sqlite")
        || lower.contains("mongodb")
}

fn looks_like_api_module(import_target: &str) -> bool {
    let lower = import_target.to_ascii_lowercase();
    lower.contains("/api") || lower.contains("route.ts") || lower.ends_with("/route")
}

fn looks_like_server_only(import_target: &str) -> bool {
    let lower = import_target.to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "fs" | "path" | "child_process" | "net" | "tls" | "server-only"
    ) || lower.contains("/server")
        || looks_like_data_layer(&lower)
}

fn is_client_file(root: &Path, file: &str, cache: &mut HashMap<String, bool>) -> Result<bool> {
    if let Some(is_client) = cache.get(file) {
        return Ok(*is_client);
    }

    let abs_path = root.join(file);
    let source = std::fs::read_to_string(&abs_path)
        .with_context(|| format!("failed to read {}", abs_path.display()))?;
    let is_client = has_use_client_directive(&source);
    cache.insert(file.to_string(), is_client);
    Ok(is_client)
}

fn parse_git_status_output(stdout: &[u8]) -> Vec<String> {
    let mut paths = Vec::new();
    let mut cursor = 0usize;

    while cursor + 3 <= stdout.len() {
        let status_x = stdout[cursor];
        let status_y = stdout[cursor + 1];
        if stdout[cursor + 2] != b' ' {
            break;
        }
        cursor += 3;

        let Some(path_end) = find_nul(stdout, cursor) else {
            break;
        };
        let path = String::from_utf8_lossy(&stdout[cursor..path_end]).into_owned();
        cursor = path_end + 1;

        let normalized = normalize_rel_path(&path);
        if !normalized.is_empty() && !is_internal_path(&normalized) {
            paths.push(path);
        }

        if is_rename_or_copy_status(status_x, status_y) {
            let Some(original_end) = find_nul(stdout, cursor) else {
                break;
            };
            cursor = original_end + 1;
        }
    }

    paths
}

fn is_rename_or_copy_status(status_x: u8, status_y: u8) -> bool {
    matches!(status_x, b'R' | b'C') || matches!(status_y, b'R' | b'C')
}

fn find_nul(bytes: &[u8], start: usize) -> Option<usize> {
    bytes[start..]
        .iter()
        .position(|byte| *byte == b'\0')
        .map(|offset| start + offset)
}

fn has_use_client_directive(source: &str) -> bool {
    let trimmed = source.trim_start_matches('\u{feff}').trim_start();
    matches_directive_prefix(trimmed, "'use client'")
        || matches_directive_prefix(trimmed, "\"use client\"")
}

fn matches_directive_prefix(source: &str, directive: &str) -> bool {
    let Some(rest) = source.strip_prefix(directive) else {
        return false;
    };

    let rest = rest.trim_start_matches([' ', '\t']);
    rest.is_empty()
        || rest.starts_with(';')
        || rest.starts_with('\n')
        || rest.starts_with('\r')
        || rest.starts_with("//")
        || rest.starts_with("/*")
}

fn normalize_rel_path(path: &str) -> String {
    path.replace('\\', "/")
}

fn is_internal_path(path: &str) -> bool {
    path == ".link" || path.starts_with(".link/") || path == ".git" || path.starts_with(".git/")
}

fn short_path(path: &str) -> &str {
    path.rsplit_once('/').map(|(_, file)| file).unwrap_or(path)
}

#[cfg(test)]
mod tests {
    use super::{has_use_client_directive, parse_git_status_output};

    #[test]
    fn test_parse_git_status_output_handles_rename_records_and_internal_paths() {
        let paths = parse_git_status_output(
            b"R  newname.ts\0oldname.ts\0?? .link/index.db\0?? .git/config\0?? src/app.ts\0",
        );
        assert_eq!(paths, vec!["newname.ts", "src/app.ts"]);
    }

    #[test]
    fn test_parse_git_status_output_preserves_literal_arrow_in_filename() {
        let paths = parse_git_status_output(b"?? file -> name.ts\0");
        assert_eq!(paths, vec!["file -> name.ts"]);
    }

    #[test]
    fn test_has_use_client_directive_is_precise() {
        assert!(has_use_client_directive(
            "'use client';\nexport default function A() {}"
        ));
        assert!(has_use_client_directive(
            "\"use client\" // comment\nexport default function A() {}"
        ));
        assert!(!has_use_client_directive(
            "'use clientish';\nexport default function A() {}"
        ));
        assert!(!has_use_client_directive(
            "const marker = 'use client';\nexport default function A() {}"
        ));
    }
}
