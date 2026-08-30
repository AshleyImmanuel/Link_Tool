use assert_cmd::prelude::*;
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

    // If show crashes with SQL error → bad escaping
    let result = cargo_link(dir, &["show", "'; DROP TABLE symbols; --"]);
    assert!(result.is_err()); // Not found, but shouldn't be a SQL panic
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[test]
fn test_editor_command_injection_attempt_via_symbol_name() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();

    cargo_link(dir, &["init"]).unwrap();

    // If show tries to run shell command with this symbol → vuln
    let result = cargo_link(dir, &["show", "$(whoami > /tmp/hacked)"]);
    assert!(result.is_err()); // should not execute shell
}

#[test]
fn test_update_runs_without_init() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path();

    // No init yet
    let result = cargo_link(dir, &["update"]);
    // Should either error nicely or do nothing — no panic
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not a Link project"));
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
