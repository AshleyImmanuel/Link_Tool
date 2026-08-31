# Contributing to LinkTool

First off, thank you for considering contributing to LinkTool! We welcome contributions from anyone, whether it's fixing a bug, adding a new language parser, or improving documentation.

## Core Philosophy

Before you start writing code, please keep our core philosophy in mind:
- **Local & Offline:** LinkTool must never require an internet connection, a cloud backend, or an API key. 
- **Simple & Fast:** We don't want background daemons or language servers. It's an on-demand static analysis tool.
- **Best-Effort:** We do not aim for 100% compiler-level accuracy if it sacrifices speed or simplicity.

## Setting Up Your Environment

You will need the following tools:
- [Rust & Cargo](https://rustup.rs/)

Clone the repository and build:
```bash
git clone https://github.com/AshleyImmanuel/Link_Tool.git
cd Link_Tool
cargo build
```

## Adding a New Language

Adding a new language is incredibly simple thanks to the `define_languages!` macro in `src/lang.rs`.

1. Add the tree-sitter grammar dependency to `Cargo.toml`:
   ```toml
   tree-sitter-mylang = "0.23"
   ```
2. Add your language to the `define_languages!` macro in `src/lang.rs`:
   ```rust
   define_languages! {
       // ... existing languages ...
       MyLang {
           extensions: ["myl", "mylang"],
           grammar: tree_sitter_mylang::language(),
           query_file: "queries/mylang.scm"
       }
   }
   ```
3. Create the Tree-sitter query file in `queries/mylang.scm` (see existing `.scm` files for examples of how to query definitions, imports, and calls).

## Submitting a Pull Request

1. Fork the repository.
2. Create a feature branch (`git checkout -b feature/amazing-feature`).
3. Ensure the code compiles and passes tests (`cargo clippy`, `cargo test`).
4. Submit your Pull Request with a clear description of the problem and your solution.

Thank you for your time and code!
