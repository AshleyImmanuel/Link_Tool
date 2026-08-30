use assert_cmd::prelude::*;
use rusqlite::Connection;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn cargo_link(dir: &Path, args: &[&str]) -> anyhow::Result<assert_cmd::assert::Assert> {
    Ok(Command::cargo_bin("link")?
        .current_dir(dir)
        .args(args)
        .assert()
        .try_success()?)
}

fn cargo_link_output(dir: &Path, args: &[&str]) -> anyhow::Result<std::process::Output> {
    Ok(Command::cargo_bin("link")?
        .current_dir(dir)
        .args(args)
        .output()?)
}

fn count_occurrences(haystack: &str, needle: &str) -> usize {
    haystack.matches(needle).count()
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
    assert!(stderr.contains("Run 'link init'"), "{stderr}");
}
