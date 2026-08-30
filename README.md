# Link

> Git for understanding code.

Link is a local CLI for exploring code structure. It parses source files with Tree-sitter, stores symbols and relationships in SQLite, and opens a small self-contained HTML viewer for graph exploration.

It is intentionally simple:

- local only, including git-aware views of the current working tree
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
- PHP

Stack-aware helpers (best-effort):

- Express route extraction: `app.get("/path", handler)` / `router.post(...)` become `route` nodes like `GET /path`
- Laravel basic routes: `Route::get('/path', 'Controller@method')` become `route` nodes like `GET /path`

## Installation

### Disclaimer

Linkmap is an **experimental hobby project** and is still under review. Use at your own risk.

If you find issues, contact Ashley via LinkedIn: `https://www.linkedin.com/in/ashley-immanuel-81609731b/`

### Install (recommended)

- Download the latest prebuilt binary from GitHub Releases.
- Verify the SHA256 checksum file shipped with the release.

### Install via `curl` (Linux/macOS)

```bash
curl -fsSL https://raw.githubusercontent.com/AshleyImmanuel/link_tool/main/install.sh | sh
```

This installs `linkmap` to `~/.local/bin/linkmap`.

### Install via package managers (Git-style)

Linkmap is easiest to install via a package manager once you publish a release.

- Windows (Scoop):

```powershell
# after you publish a Scoop bucket repo:
scoop bucket add linkmap https://github.com/AshleyImmanuel/linkmap-scoop-bucket
scoop install linkmap
```

- macOS/Linux (Homebrew):

```bash
# after you publish a Homebrew tap repo:
brew tap ashleyimmanuel/linkmap
brew install linkmap
```

### Install from source (Cargo)

Install Rust and Cargo from [rustup.rs](https://rustup.rs/), then build Link from source:

```bash
cargo build --release
```

The binary will be available at:

```bash
target/release/linkmap
```

You can also install it into Cargo's bin directory:

```bash
cargo install --path .
```

### Versioning and releases

- Public releases are tagged as `vX.Y.Z`.
- Release artifacts include `.zip`/`.tar.gz` packages plus a `SHA256SUMS` file.

## Quick Start

Run Link from the root of the repository you want to inspect.

```bash
linkmap init
linkmap search calculate
linkmap show calculate
linkmap history
linkmap update
```

`linkmap init` creates a local index at `.link/index.db`.

`linkmap show <symbol>` opens `.link/show.html` in your browser. Double-clicking a node attempts to open the corresponding file in VS Code via the `vscode://file/...` URI scheme.

## Upgrade and Rebuild

`.link/index.db` is a local cache, not source of truth.

If you upgrade Link and it reports that the index format is out of date, run:

```bash
linkmap init
```

`linkmap update` refreshes an existing compatible index. It is not a migration command.

## Command Reference

### `linkmap init`

Scans the current directory, indexes supported source files, and rebuilds `.link/index.db`.

### `linkmap show <symbol>`

Shows the caller/callee neighborhood for an indexed symbol.

- default mode: opens the HTML graph viewer
- `--json`: prints the graph payload to stdout

### `linkmap list`

Lists indexed definition symbols and their locations.

### `linkmap search <query>`

Performs a name-based fuzzy search against indexed symbols.

### `linkmap update`

Re-indexes only changed, new, and deleted files, then rebuilds relationships.

### `linkmap snapshot`

Writes a portable structure snapshot to a JSON file.

- default path: `.link/snapshot.json`
- `--out <path>`: write to a custom path

### `linkmap diff <from> <to>`

Shows a structural diff between two snapshot files.

- default mode: prints a human-friendly change report
- `--json`: prints the diff payload as JSON

### Optional commands

- `linkmap history`: project-local command history, with current-session filtering when a shell session id is available
- `linkmap stats`: index metrics, heuristic architecture checks, and local git-aware change summaries
- `linkmap explain <symbol>`: text explanations of symbol connections, impact hints, and heuristic warnings

These remain lightweight helpers and are not required for normal use.

### `linkmap history`

Shows recent `linkmap` commands recorded for the current project.

- By default, it uses the current shell session when Link can detect one from environment markers such as `LINK_SESSION_ID`, `WT_SESSION`, or `TERM_SESSION_ID`.
- `linkmap history --all` shows the full recorded project history.
- History is stored locally in `.link/index.db` and survives `link init`.

### `linkmap stats`

Shows index counts plus two local-only helpers:

- `Architecture Rules` are built-in heuristic import checks for common UI/server layering issues.
- `Change Summary` compares the local git working tree to `HEAD` using extracted symbol, import, call, and render signatures.

These sections are intentionally lightweight. They are not a configurable policy engine or a full semantic diff.

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
- Architecture rules are built-in heuristics, not a full configurable policy engine
- Git-aware change summaries compare the local working tree to `HEAD`; they do not inspect remotes or push state
- Change summaries diff extracted symbols/imports/calls/renders, so they are structural hints rather than full semantic analysis
- Session-scoped command history depends on shell or terminal session ids being available; otherwise Link falls back to all recorded project history

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
