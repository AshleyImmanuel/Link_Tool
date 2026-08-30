use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    JavaScript,
    TypeScript,
    Python,
    Go,
    Rust,
}

impl Lang {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "js" | "jsx" | "mjs" | "cjs" => Some(Lang::JavaScript),
            "ts" | "tsx" | "mts" | "cts" => Some(Lang::TypeScript),
            "py" | "pyi" => Some(Lang::Python),
            "go" => Some(Lang::Go),
            "rs" => Some(Lang::Rust),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Lang::JavaScript => "javascript",
            Lang::TypeScript => "typescript",
            Lang::Python => "python",
            Lang::Go => "go",
            Lang::Rust => "rust",
        }
    }

    pub fn ts_language(&self) -> tree_sitter::Language {
        match self {
            Lang::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            Lang::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Lang::Python => tree_sitter_python::LANGUAGE.into(),
            Lang::Go => tree_sitter_go::LANGUAGE.into(),
            Lang::Rust => tree_sitter_rust::LANGUAGE.into(),
        }
    }
}

impl std::fmt::Display for Lang {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// Directories to always skip during scanning.
const SKIP_DIRS: &[&str] = &[
    ".git",
    ".link",
    ".hg",
    ".svn",
    "node_modules",
    "__pycache__",
    ".venv",
    "venv",
    "target",
    "vendor",
    "dist",
    "build",
    ".next",
    ".nuxt",
    ".tox",
    "coverage",
];

/// Maximum file size to process (1 MB).
const MAX_FILE_SIZE: u64 = 1_048_576;

/// Detect language for a file path. Returns None if unsupported.
pub fn detect_lang(path: &Path) -> Option<Lang> {
    path.extension()
        .and_then(|e| e.to_str())
        .and_then(Lang::from_extension)
}

/// Check if a directory entry should be skipped.
pub fn should_skip_dir(name: &str) -> bool {
    SKIP_DIRS.contains(&name)
}

/// Check if a file is too large to process.
pub fn is_too_large(path: &Path) -> bool {
    path.metadata().map(|m| m.len() > MAX_FILE_SIZE).unwrap_or(false)
}
