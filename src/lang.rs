use std::path::Path;

macro_rules! define_languages {
    (
        $(
            $variant:ident {
                name: $name:expr,
                extensions: [$($ext:expr),*],
                grammar: $grammar:expr,
                query: $query:expr
            }
        ),* $(,)?
    ) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum Lang {
            $($variant),*
        }

        impl Lang {
            pub fn from_extension(ext: &str) -> Option<Self> {
                match ext {
                    $($($ext)|* => Some(Lang::$variant),)*
                    _ => None,
                }
            }

            pub fn name(&self) -> &'static str {
                match self {
                    $(Lang::$variant => $name),*
                }
            }

            pub fn ts_language(&self) -> tree_sitter::Language {
                match self {
                    $(Lang::$variant => $grammar.into()),*
                }
            }

            pub fn query_str(&self) -> &'static str {
                match self {
                    $(Lang::$variant => include_str!($query)),*
                }
            }
        }
    };
}

define_languages! {
    JavaScript {
        name: "javascript",
        extensions: ["js", "jsx", "mjs", "cjs"],
        grammar: tree_sitter_javascript::LANGUAGE,
        query: "../queries/javascript.scm"
    },
    TypeScript {
        name: "typescript",
        extensions: ["ts", "mts", "cts"],
        grammar: tree_sitter_typescript::LANGUAGE_TYPESCRIPT,
        query: "../queries/typescript.scm"
    },
    Tsx {
        name: "tsx",
        extensions: ["tsx"],
        grammar: tree_sitter_typescript::LANGUAGE_TSX,
        query: "../queries/tsx.scm"
    },
    Python {
        name: "python",
        extensions: ["py", "pyi"],
        grammar: tree_sitter_python::LANGUAGE,
        query: "../queries/python.scm"
    },
    Go {
        name: "go",
        extensions: ["go"],
        grammar: tree_sitter_go::LANGUAGE,
        query: "../queries/go.scm"
    },
    Rust {
        name: "rust",
        extensions: ["rs"],
        grammar: tree_sitter_rust::LANGUAGE,
        query: "../queries/rust.scm"
    },
    Php {
        name: "php",
        extensions: ["php"],
        grammar: tree_sitter_php::LANGUAGE_PHP,
        query: "../queries/php.scm"
    },
    Java {
        name: "java",
        extensions: ["java"],
        grammar: tree_sitter_java::LANGUAGE,
        query: "../queries/java.scm"
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

pub fn max_file_size() -> u64 {
    MAX_FILE_SIZE
}
