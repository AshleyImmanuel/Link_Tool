use std::path::Path;

use anyhow::{Context, Result};

use crate::db::Db;
use crate::error::user_error;
use crate::snapshot;

pub fn run(out: Option<&str>, quiet: bool) -> Result<()> {
    let cwd = std::env::current_dir().context("failed to get current directory")?;
    let link_dir = cwd.join(".link");
    let db_path = link_dir.join("index.db");

    if !db_path.exists() {
        return Err(user_error("not a Link project. Run 'linkmap init' first."));
    }

    let db = Db::open_index(&link_dir)?;
    let snapshot = snapshot::build_snapshot(&db, Some(&cwd))?;
    let out_path = match out {
        Some(path) => Path::new(path).to_path_buf(),
        None => snapshot::default_snapshot_path(&link_dir),
    };
    snapshot::write_snapshot(&out_path, &snapshot)?;

    if !quiet {
        println!(
            "Wrote snapshot {}\n  {} symbols | {} edges | {} imports",
            out_path.display(),
            snapshot.symbols.len(),
            snapshot.edges.len(),
            snapshot.import_refs.len()
        );
    }

    Ok(())
}
