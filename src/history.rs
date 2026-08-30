use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;

use crate::db::Db;

const GLOBAL_SESSION_KEY: &str = "global";

#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub key: Option<String>,
    pub source: Option<&'static str>,
}

pub fn detect_session() -> SessionInfo {
    for (name, source) in [
        ("LINK_SESSION_ID", "LINK_SESSION_ID"),
        ("WT_SESSION", "WT_SESSION"),
        ("TERM_SESSION_ID", "TERM_SESSION_ID"),
        ("ConEmuPID", "ConEmuPID"),
        ("TMUX", "TMUX"),
        ("STY", "STY"),
    ] {
        if let Some(value) = env_value(name) {
            return SessionInfo {
                key: Some(format!("{source}:{value}")),
                source: Some(source),
            };
        }
    }

    SessionInfo {
        key: None,
        source: None,
    }
}

pub fn record_command(link_dir: &Path, cwd: &Path, invocation: &str, success: bool) -> Result<()> {
    if !link_dir.join("index.db").exists() {
        return Ok(());
    }

    let db = Db::open(link_dir)?;
    let session = detect_session();
    let session_key = session
        .key
        .unwrap_or_else(|| GLOBAL_SESSION_KEY.to_string());
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    db.insert_command_history(
        timestamp,
        &session_key,
        &cwd.display().to_string(),
        invocation,
        success,
    )
}

pub fn format_invocation(args: &[String]) -> String {
    if args.is_empty() {
        return "linkmap".to_string();
    }

    let mut rendered = String::from("linkmap");
    for arg in args {
        rendered.push(' ');
        rendered.push_str(&quote_arg(arg));
    }
    rendered
}

pub fn scope_label(all: bool, session: &SessionInfo) -> String {
    if all {
        "all recorded commands for this project".to_string()
    } else if let Some(source) = session.source {
        format!("current session ({source})")
    } else {
        "all recorded commands for this project (no shell session id detected)".to_string()
    }
}

pub fn format_age(ts: u64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let delta = now.saturating_sub(ts);

    if delta < 60 {
        format!("{}s ago", delta)
    } else if delta < 3600 {
        format!("{}m ago", delta / 60)
    } else if delta < 86_400 {
        format!("{}h ago", delta / 3600)
    } else {
        format!("{}d ago", delta / 86_400)
    }
}

pub fn display_session_key(session_key: &str) -> String {
    if session_key == GLOBAL_SESSION_KEY {
        return "global".to_string();
    }

    match session_key.split_once(':') {
        Some((source, value)) if value.len() > 16 => {
            format!("{source}:{}...", &value[..16])
        }
        Some((source, value)) => format!("{source}:{value}"),
        None => session_key.to_string(),
    }
}

pub fn has_exact_session(session: &SessionInfo) -> bool {
    session.key.is_some()
}

fn env_value(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn quote_arg(arg: &str) -> String {
    if arg.is_empty() {
        return "\"\"".to_string();
    }

    if arg
        .chars()
        .any(|ch| ch.is_whitespace() || matches!(ch, '"' | '\''))
    {
        return format!("\"{}\"", arg.replace('\\', "\\\\").replace('"', "\\\""));
    }

    arg.to_string()
}
