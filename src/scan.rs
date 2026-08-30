use anyhow::Result;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::lang::{self, Lang};

const LARGE_REPO_WARNING_THRESHOLD: usize = 10_000;

#[derive(Debug, Clone)]
pub struct SourceFile {
    pub abs_path: PathBuf,
    pub rel_path: String,
    pub lang: Lang,
    pub last_modified: i64,
}

pub fn collect_source_files(root: &Path, quiet: bool) -> Result<Vec<SourceFile>> {
    let mut files = Vec::new();
    let mut warned_large_repo = false;

    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            if entry.file_type().is_dir() {
                let name = entry.file_name().to_str().unwrap_or("");
                !lang::should_skip_dir(name)
            } else {
                true
            }
        })
    {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                if !quiet {
                    eprintln!("warning: failed to read directory entry: {err}");
                }
                continue;
            }
        };

        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.into_path();
        let Some(lang) = lang::detect_lang(&path) else {
            continue;
        };

        let metadata = match path.metadata() {
            Ok(metadata) => metadata,
            Err(err) => {
                if !quiet {
                    eprintln!(
                        "warning: failed to read file metadata {}: {err}",
                        path.display()
                    );
                }
                continue;
            }
        };

        if metadata.len() > lang::max_file_size() {
            if !quiet {
                eprintln!("warning: skipping large file {}", path.display());
            }
            continue;
        }

        let rel_path = match path.strip_prefix(root) {
            Ok(rel) => rel.to_string_lossy().replace('\\', "/"),
            Err(err) => {
                if !quiet {
                    eprintln!(
                        "warning: failed to normalize path {}: {err}",
                        path.display()
                    );
                }
                continue;
            }
        };

        let last_modified = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duration| duration.as_secs() as i64)
            .unwrap_or(0);

        files.push(SourceFile {
            abs_path: path,
            rel_path,
            lang,
            last_modified,
        });

        if !warned_large_repo && files.len() > LARGE_REPO_WARNING_THRESHOLD {
            warned_large_repo = true;
            if !quiet {
                eprintln!("warning: large repository detected ({} files)", files.len());
            }
        }
    }

    Ok(files)
}
