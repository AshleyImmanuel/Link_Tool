# Link

> Git for understanding code.

Link is a local CLI for exploring code structure. It parses source files with Tree-sitter, stores symbols and relationships in SQLite, and opens a small self-contained HTML viewer for graph exploration.

It is intentionally simple:

- local only
- offline only
- best-effort static analysis
- no servers, agents, or background daemons

## What Link Does

Link indexes definitions, calls, and imports, then lets you:

- initialize an index for the current repository
- list known symbols
- search symbols by name
- inspect a symbol graph
- incrementally update the index after code changes

Supported languages:

- JavaScript
- TypeScript
- Python
- Go
- Rust

## Installation

Install Rust and Cargo from [rustup.rs](https://rustup.rs/), then build Link from source:

```bash
cargo build --release
```

The binary will be available at:

```bash
target/release/link
```

You can also install it into Cargo's bin directory:

```bash
cargo install --path .
```

## Quick Start

Run Link from the root of the repository you want to inspect.

```bash
link init
link search calculate
link show calculate
link update
```

`link init` creates a local index at `.link/index.db`.

`link show <symbol>` opens `.link/show.html` in your browser. Double-clicking a node attempts to open the corresponding file in VS Code via the `vscode://file/...` URI scheme.

## Upgrade and Rebuild

`.link/index.db` is a local cache, not source of truth.

If you upgrade Link and it reports that the index format is out of date, run:

```bash
link init
```

`link update` refreshes an existing compatible index. It is not a migration command.

## Command Reference

### `link init`

Scans the current directory, indexes supported source files, and rebuilds `.link/index.db`.

### `link show <symbol>`

Shows the caller/callee neighborhood for an indexed symbol.

- default mode: opens the HTML graph viewer
- `--json`: prints the graph payload to stdout

### `link list`

Lists indexed definition symbols and their locations.

### `link search <query>`

Performs a name-based fuzzy search against indexed symbols.

### `link update`

Re-indexes only changed, new, and deleted files, then rebuilds relationships.

### Optional commands

- `link stats`
- `link explain <symbol>`

These remain lightweight helpers and are not required for normal use.

## Exit Codes

Link uses stable process exit codes:

- `0`: success
- `1`: user error, such as running `link update` before `link init`
- `2`: internal/runtime failure

## How It Works

Link uses a small static-analysis pipeline:

1. Walk the repository and pick supported files.
2. Parse each file with Tree-sitter.
3. Run language-specific query files in `queries/*.scm`.
4. Store symbols and relationships in SQLite.
5. Resolve cross-file references using simple structural heuristics.

The analysis is intentionally conservative. Link prefers skipping ambiguous edges over inventing connections.

## Limits and Safeguards

Link is hardened for local use on untrusted repositories.

- It only writes inside `.link/`.
- It does not follow symlinks while scanning.
- It skips files larger than 1 MB.
- It skips common dependency/build directories such as `.git`, `node_modules`, `target`, `dist`, `build`, `vendor`, `.venv`, and `__pycache__`.
- It warns when more than 10,000 supported files are detected.
- Parse failures are reported as warnings and do not abort the run.

Example warning:

```text
warning: failed to parse file src/parser.rs
```

## Limitations

Link is best-effort static analysis, not a type checker or runtime tracer.

- No type inference
- No runtime dispatch analysis
- No reflection or `eval` support
- No guarantee that every call/import can be resolved
- Ambiguous matches are skipped on purpose

This means results are useful for navigation and architecture understanding, but they are not a proof of program behavior.

## Large Repository Behavior

Link is designed to stay predictable on larger repositories.

- file scanning is sequential
- files are parsed one at a time
- indexing writes run inside SQLite transactions
- updates avoid buffering full file contents for the whole repo

If the repository is extremely large, expect reduced throughput, but Link should continue operating without crashing on malformed files.

## Adding a New Language

To add a language:

1. Add a Tree-sitter grammar dependency in `Cargo.toml`.
2. Extend `src/lang.rs` with the file extension mapping and grammar handle.
3. Add a query file under `queries/<language>.scm`.
4. Update `src/parser.rs` to expose the query for that language.
5. Add examples and tests for the new extractor behavior.

Keep queries focused on:

- definitions
- calls
- imports

Link works best when each language integration stays small and syntax-driven.
