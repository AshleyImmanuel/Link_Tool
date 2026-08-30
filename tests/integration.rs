use assert_cmd::prelude::*;
use rusqlite::Connection;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn cargo_link(dir: &Path, args: &[&str]) -> anyhow::Result<assert_cmd::assert::Assert> {
    Ok(Command::cargo_bin("linkmap")?
        .current_dir(dir)
        .args(args)
        .assert()
        .try_success()?)
}

fn cargo_link_output(dir: &Path, args: &[&str]) -> anyhow::Result<std::process::Output> {
    Ok(Command::cargo_bin("linkmap")?
        .current_dir(dir)
        .args(args)
        .output()?)
}

fn cargo_link_output_with_session(
    dir: &Path,
    args: &[&str],
    session: Option<&str>,
) -> anyhow::Result<std::process::Output> {
    let mut command = Command::cargo_bin("linkmap")?;
    command.current_dir(dir).args(args);
    match session {
        Some(session) => {
            command.env("LINK_SESSION_ID", session);
        }
        None => {
            command.env_remove("LINK_SESSION_ID");
        }
    }
    Ok(command.output()?)
}

fn count_occurrences(haystack: &str, needle: &str) -> usize {
    haystack.matches(needle).count()
}

fn init_git_repo(dir: &Path) {
    Command::new("git")
        .current_dir(dir)
        .args(["init"])
        .assert()
        .success();
    Command::new("git")
        .current_dir(dir)
        .args(["config", "user.email", "link@example.com"])
        .assert()
        .success();
    Command::new("git")
        .current_dir(dir)
        .args(["config", "user.name", "Link Test"])
        .assert()
        .success();
    Command::new("git")
        .current_dir(dir)
        .args(["add", "."])
        .assert()
        .success();
    Command::new("git")
        .current_dir(dir)
        .args(["commit", "-m", "init"])
        .assert()
        .success();
}

#[test]
fn test_snapshot_writes_default_file() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();

    std::fs::write(
        dir.join("app.py"),
        r#"def add(a, b):
    return a + b
"#,
    )
    .unwrap();

    cargo_link(dir, &["init"]).unwrap();
    cargo_link(dir, &["snapshot", "--quiet"]).unwrap();
    assert!(dir.join(".link/snapshot.json").exists());
}

#[test]
fn test_diff_reports_added_symbol_after_change() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();

    std::fs::write(
        dir.join("app.py"),
        r#"def add(a, b):
    return a + b
"#,
    )
    .unwrap();

    cargo_link(dir, &["init", "--quiet"]).unwrap();
    cargo_link(dir, &["snapshot", "--out", "a.json", "--quiet"]).unwrap();

    std::fs::write(
        dir.join("app.py"),
        r#"def add(a, b):
    return a + b

def subtract(a, b):
    return a - b
"#,
    )
    .unwrap();

    cargo_link(dir, &["update", "--quiet"]).unwrap();
    cargo_link(dir, &["snapshot", "--out", "b.json", "--quiet"]).unwrap();

    let output = cargo_link_output(dir, &["diff", "a.json", "b.json"]).unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Added symbols:") || stdout.contains("+ subtract"),
        "{stdout}"
    );
}

#[test]
fn test_express_route_is_indexed_and_links_to_handler() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();

    std::fs::write(
        dir.join("server.js"),
        r#"const express = require("express");
const app = express();

function hello(req, res) {
  res.send("ok");
}

app.get("/hello", hello);
"#,
    )
    .unwrap();

    cargo_link(dir, &["init", "--quiet"]).unwrap();

    let output = cargo_link_output(dir, &["show", "GET /hello", "--json"]).unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"name\": \"GET /hello\""), "{stdout}");
    assert!(stdout.contains("\"label\": \"routes_to\""), "{stdout}");
    assert!(stdout.contains("hello"), "{stdout}");
}

#[test]
fn test_laravel_route_is_indexed_and_links_to_controller_method() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();

    std::fs::create_dir_all(dir.join("routes")).unwrap();
    std::fs::create_dir_all(dir.join("app/Http/Controllers")).unwrap();

    std::fs::write(
        dir.join("routes/web.php"),
        r#"<?php
Route::get('/hello', 'HelloController@index');
"#,
    )
    .unwrap();

    std::fs::write(
        dir.join("app/Http/Controllers/HelloController.php"),
        r#"<?php
class HelloController {
  public function index() {
    return "ok";
  }
}
"#,
    )
    .unwrap();

    cargo_link(dir, &["init", "--quiet"]).unwrap();
    let output = cargo_link_output(dir, &["show", "GET /hello", "--json"]).unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"name\": \"GET /hello\""), "{stdout}");
    assert!(stdout.contains("\"label\": \"routes_to\""), "{stdout}");
    assert!(stdout.contains("index"), "{stdout}");
}

#[test]
fn test_init_creates_dot_link_folder_and_db() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();

    cargo_link(dir, &["init"]).unwrap();

    assert!(dir.join(".link").exists());
    assert!(dir.join(".link/index.db").exists());
}

#[test]
fn test_show_non_existing_symbol_gives_nice_message() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();

    cargo_link(dir, &["init"]).unwrap();

    let output = cargo_link(dir, &["show", "NonExistentSymbol123"])
        .unwrap_err()
        .to_string();

    assert!(output.contains("not found"));
}

#[test]
fn test_init_does_not_crash_on_empty_folder() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();

    let result = cargo_link(dir, &["init"]);
    assert!(result.is_ok());
}

#[test]
fn test_symbol_with_sql_like_name_does_not_inject() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();

    cargo_link(dir, &["init"]).unwrap();

    let result = cargo_link(dir, &["show", "'; DROP TABLE symbols; --"]);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[test]
fn test_editor_command_injection_attempt_via_symbol_name() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();

    cargo_link(dir, &["init"]).unwrap();

    let result = cargo_link(dir, &["show", "$(whoami > /tmp/hacked)"]);
    assert!(result.is_err());
}

#[test]
fn test_update_runs_without_init() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();

    let result = cargo_link(dir, &["update"]);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("not a Link project"));
}

#[test]
fn test_list_does_not_panic_on_fresh_init() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();

    cargo_link(dir, &["init"]).unwrap();
    let result = cargo_link(dir, &["list"]);
    assert!(result.is_ok());
}

#[test]
fn test_search_with_special_chars() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();

    cargo_link(dir, &["init"]).unwrap();

    let result = cargo_link(dir, &["search", "O'Reilly OR 1=1"]);
    assert!(result.is_ok());
}

#[test]
fn test_user_errors_exit_with_code_one() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();

    let output = cargo_link_output(dir, &["update"]).unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("not a Link project"));
}

#[test]
fn test_init_skips_large_files_with_warning() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();
    let large_file = dir.join("huge.js");
    let content = vec![b'a'; 1_048_577];
    std::fs::write(&large_file, content).unwrap();

    let output = cargo_link_output(dir, &["init"]).unwrap();
    assert!(output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("warning: skipping large file"));
}

#[test]
fn test_update_does_not_duplicate_edges() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();

    std::fs::copy("examples/test_repo/app.py", dir.join("app.py")).unwrap();
    std::fs::copy("examples/test_repo/helpers.py", dir.join("helpers.py")).unwrap();
    std::fs::copy("examples/test_repo/main.js", dir.join("main.js")).unwrap();
    std::fs::copy("examples/test_repo/utils.js", dir.join("utils.js")).unwrap();

    cargo_link(dir, &["init"]).unwrap();
    let before = cargo_link_output(dir, &["show", "add", "--json"]).unwrap();
    assert!(before.status.success());
    let before_stdout = String::from_utf8_lossy(&before.stdout);
    let before_edges = count_occurrences(&before_stdout, "\"from\"");

    cargo_link(dir, &["update"]).unwrap();
    let after = cargo_link_output(dir, &["show", "add", "--json"]).unwrap();
    assert!(after.status.success());
    let after_stdout = String::from_utf8_lossy(&after.stdout);
    let after_edges = count_occurrences(&after_stdout, "\"from\"");

    assert_eq!(before_edges, after_edges);
}

#[test]
fn test_show_component_in_tsx_repo_has_import_edge() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();

    std::fs::create_dir_all(dir.join("app")).unwrap();
    std::fs::create_dir_all(dir.join("components")).unwrap();

    std::fs::write(
        dir.join("components/HeroSection.tsx"),
        r#"const HeroSection = () => {
  return <section>Hero</section>;
};

export default HeroSection;
"#,
    )
    .unwrap();

    std::fs::write(
        dir.join("app/page.tsx"),
        r#"import HeroSection from "../components/HeroSection";

export default function Home() {
  return <HeroSection />;
}
"#,
    )
    .unwrap();

    cargo_link(dir, &["init"]).unwrap();
    let output = cargo_link_output(dir, &["show", "HeroSection", "--json"]).unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(count_occurrences(&stdout, "\"from\"") > 0, "{stdout}");
    assert!(stdout.contains("\"label\": \"imports\""), "{stdout}");
}

#[test]
fn test_read_commands_require_current_index_format() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();

    std::fs::copy("examples/test_repo/app.py", dir.join("app.py")).unwrap();
    cargo_link(dir, &["init"]).unwrap();

    let conn = Connection::open(dir.join(".link/index.db")).unwrap();
    conn.execute("DELETE FROM meta WHERE key = 'index_format_version'", [])
        .unwrap();

    let output = cargo_link_output(dir, &["list"]).unwrap();
    assert_eq!(output.status.code(), Some(1));

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Run 'linkmap init'"), "{stderr}");
}

#[test]
fn test_explain_supports_path_queries() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();

    std::fs::create_dir_all(dir.join("app")).unwrap();
    std::fs::create_dir_all(dir.join("components")).unwrap();

    std::fs::write(
        dir.join("components/HeroSection.tsx"),
        r#"const HeroSection = () => {
  return <section>Hero</section>;
};

export default HeroSection;
"#,
    )
    .unwrap();

    std::fs::write(
        dir.join("app/page.tsx"),
        r#"import HeroSection from "../components/HeroSection";

export default function Home() {
  return <HeroSection />;
}
"#,
    )
    .unwrap();

    cargo_link(dir, &["init"]).unwrap();
    let output = cargo_link_output(dir, &["explain", "page.tsx -> HeroSection"]).unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Path:"), "{stdout}");
    assert!(
        stdout.contains("--imports-->") || stdout.contains("--renders-->"),
        "{stdout}"
    );
}

#[test]
fn test_stats_reports_architecture_violations() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();

    std::fs::create_dir_all(dir.join("components/ui")).unwrap();
    std::fs::write(
        dir.join("components/ui/Button.tsx"),
        r#"'use client';

import fs from "fs";

const Button = () => {
  return <button>Click</button>;
};

export default Button;
"#,
    )
    .unwrap();

    cargo_link(dir, &["init"]).unwrap();
    let output = cargo_link_output(dir, &["stats"]).unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Architecture Rules"), "{stdout}");
    assert!(
        stdout.contains("built-in heuristic import checks"),
        "{stdout}"
    );
    assert!(stdout.contains("client-no-server"), "{stdout}");
}

#[test]
fn test_stats_reports_git_changes() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();

    std::fs::write(
        dir.join("app.py"),
        r#"def add(a, b):
    return a + b
"#,
    )
    .unwrap();

    init_git_repo(dir);

    cargo_link(dir, &["init"]).unwrap();
    std::fs::write(
        dir.join("app.py"),
        r#"def add(a, b):
    return a + b

def subtract(a, b):
    return a - b
"#,
    )
    .unwrap();

    let output = cargo_link_output(dir, &["stats"]).unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Change Summary"), "{stdout}");
    assert!(
        stdout.contains("local git working tree vs HEAD"),
        "{stdout}"
    );
    assert!(stdout.contains("not full semantic diff"), "{stdout}");
    assert!(stdout.contains("Changed files: 1"), "{stdout}");
}

#[test]
fn test_help_mentions_new_stats_and_explain_scope() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();

    let output = cargo_link_output(dir, &["--help"]).unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("stats") && stdout.contains("Show index stats, heuristic rules"),
        "{stdout}"
    );
    assert!(
        stdout.contains("explain")
            && stdout.contains("Explain a symbol's connections and local impact hints"),
        "{stdout}"
    );
}

#[test]
fn test_explain_symbol_reports_changed_file_and_heuristic_violation() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();

    std::fs::create_dir_all(dir.join("components/ui")).unwrap();
    std::fs::write(
        dir.join("components/ui/Button.tsx"),
        r#"'use client';

import fs from "fs";

export default function Button() {
  return <button>Click</button>;
}
"#,
    )
    .unwrap();

    init_git_repo(dir);
    cargo_link(dir, &["init"]).unwrap();

    std::fs::write(
        dir.join("components/ui/Button.tsx"),
        r#"'use client';

import fs from "fs";

export default function Button() {
  return <button>Click now</button>;
}
"#,
    )
    .unwrap();

    let output = cargo_link_output(dir, &["explain", "Button"]).unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Changed in local git working tree (vs HEAD)"),
        "{stdout}"
    );
    assert!(stdout.contains("Rule violations (heuristic):"), "{stdout}");
    assert!(stdout.contains("client-no-server"), "{stdout}");
}

#[test]
fn test_show_json_includes_metadata_for_changed_nodes() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();

    std::fs::create_dir_all(dir.join("app")).unwrap();
    std::fs::create_dir_all(dir.join("components")).unwrap();

    std::fs::write(
        dir.join("components/HeroSection.tsx"),
        r#"const HeroSection = () => {
  return <section>Hero</section>;
};

export default HeroSection;
"#,
    )
    .unwrap();

    std::fs::write(
        dir.join("app/page.tsx"),
        r#"import HeroSection from "../components/HeroSection";

export default function Home() {
  return <HeroSection />;
}
"#,
    )
    .unwrap();

    init_git_repo(dir);
    cargo_link(dir, &["init"]).unwrap();

    std::fs::write(
        dir.join("app/page.tsx"),
        r#"import HeroSection from "../components/HeroSection";

export default function Home() {
  return <main><HeroSection /></main>;
}
"#,
    )
    .unwrap();

    let output = cargo_link_output(dir, &["show", "HeroSection", "--json"]).unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"title\":"), "{stdout}");
    assert!(stdout.contains("\"impact_depth\":"), "{stdout}");
    assert!(stdout.contains("\"changed\": true"), "{stdout}");
}

#[test]
fn test_stats_reports_all_heuristic_rule_types() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();

    std::fs::create_dir_all(dir.join("components")).unwrap();
    std::fs::create_dir_all(dir.join("lib")).unwrap();
    std::fs::create_dir_all(dir.join("app/api/users")).unwrap();
    std::fs::create_dir_all(dir.join("app")).unwrap();

    std::fs::write(
        dir.join("components/Card.tsx"),
        r#"import db from "../lib/db";

export default function Card() {
  return <div>Card</div>;
}
"#,
    )
    .unwrap();

    std::fs::write(dir.join("lib/db.ts"), "export const db = {};\n").unwrap();

    std::fs::write(
        dir.join("app/api/users/route.ts"),
        r#"export async function GET() {
  return new Response("ok");
}
"#,
    )
    .unwrap();

    std::fs::write(
        dir.join("app/page.tsx"),
        r#"import usersRoute from "./api/users/route";

export default function Home() {
  return <div>Home</div>;
}
"#,
    )
    .unwrap();

    cargo_link(dir, &["init"]).unwrap();
    let output = cargo_link_output(dir, &["stats"]).unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("ui-no-db"), "{stdout}");
    assert!(stdout.contains("route-no-api-import"), "{stdout}");
}

#[test]
fn test_stats_does_not_treat_similar_directive_as_use_client() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();

    std::fs::create_dir_all(dir.join("components/ui")).unwrap();
    std::fs::write(
        dir.join("components/ui/Button.tsx"),
        r#"'use clientish';

import fs from "fs";

export default function Button() {
  return <button>Click</button>;
}
"#,
    )
    .unwrap();

    cargo_link(dir, &["init"]).unwrap();
    let output = cargo_link_output(dir, &["stats"]).unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Architecture Rules"), "{stdout}");
    assert!(!stdout.contains("client-no-server"), "{stdout}");
}

#[test]
fn test_history_tracks_current_session_commands() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();

    let output =
        cargo_link_output_with_session(dir, &["init", "--quiet"], Some("session-a")).unwrap();
    assert!(output.status.success());

    let output =
        cargo_link_output_with_session(dir, &["search", "alpha", "--quiet"], Some("session-a"))
            .unwrap();
    assert!(output.status.success());

    let output =
        cargo_link_output_with_session(dir, &["search", "beta", "--quiet"], Some("session-b"))
            .unwrap();
    assert!(output.status.success());

    let output = cargo_link_output_with_session(dir, &["history"], Some("session-a")).unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("current session (LINK_SESSION_ID)"),
        "{stdout}"
    );
    assert!(stdout.contains("linkmap init --quiet"), "{stdout}");
    assert!(stdout.contains("linkmap search alpha --quiet"), "{stdout}");
    assert!(!stdout.contains("linkmap search beta --quiet"), "{stdout}");
}

#[test]
fn test_history_all_shows_commands_across_sessions_and_survives_reinit() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();

    let output =
        cargo_link_output_with_session(dir, &["init", "--quiet"], Some("session-a")).unwrap();
    assert!(output.status.success());

    let output =
        cargo_link_output_with_session(dir, &["search", "alpha", "--quiet"], Some("session-b"))
            .unwrap();
    assert!(output.status.success());

    let output =
        cargo_link_output_with_session(dir, &["init", "--quiet"], Some("session-a")).unwrap();
    assert!(output.status.success());

    let output =
        cargo_link_output_with_session(dir, &["history", "--all", "--limit", "10"], Some("x"))
            .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("all recorded commands for this project"),
        "{stdout}"
    );
    assert!(stdout.contains("[LINK_SESSION_ID:session-a]"), "{stdout}");
    assert!(stdout.contains("[LINK_SESSION_ID:session-b]"), "{stdout}");
    assert!(stdout.contains("linkmap init --quiet"), "{stdout}");
    assert!(stdout.contains("linkmap search alpha --quiet"), "{stdout}");
}
