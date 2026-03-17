# Linkmap user manual

Linkmap is a local CLI that indexes code structure (best-effort static analysis) and lets you explore it as a graph.

## Quick start

From your repository root:

```bash
linkmap init
linkmap search <query>
linkmap show <symbol>
linkmap update
```

- `linkmap init`: creates/rebuilds `.link/index.db`
- `linkmap update`: incrementally re-indexes changed files
- `linkmap show`: opens the HTML viewer at `.link/show.html`

## Mental model

Linkmap stores two core things:

- **Symbols**: definitions and extracted items like routes/handlers/components
- **Edges**: relationships like calls/imports/renders/routes-to

Linkmap is intentionally conservative: it prefers **skipping ambiguous edges** over guessing.

## Commands

### `linkmap init`

Build a fresh index for the current repo:

```bash
linkmap init
```

Notes:

- Writes only inside `.link/`
- Skips common build/vendor directories
- Skips large files (safety + performance)

### `linkmap update`

Refresh an existing index by re-indexing changed/new/deleted files:

```bash
linkmap update
```

If you see an “index format out of date” message, run `linkmap init`.

### `linkmap list`

List indexed definition symbols:

```bash
linkmap list
```

### `linkmap search <query>`

Fuzzy search symbols by name:

```bash
linkmap search router
linkmap search HeroSection
```

### `linkmap show <symbol>`

Open the graph viewer centered on a symbol:

```bash
linkmap show generate_html
```

Options:

- `--json`: print graph payload as JSON instead of opening the viewer

Viewer tips:

- Double-click a node to open it in VS Code (via `vscode://file/...`)
- Use the search box to jump to a node

### `linkmap snapshot`

Write a portable structure snapshot:

```bash
linkmap snapshot
linkmap snapshot --out my-snapshot.json
```

Default output is `.link/snapshot.json`.

### `linkmap diff <from> <to>`

Compare two snapshots:

```bash
linkmap diff .link/snapshot.json other.json
```

Options:

- `--json`: print diff payload as JSON

### `linkmap history`

Show project-local command history stored in `.link/index.db`:

```bash
linkmap history
linkmap history --all
```

### `linkmap stats`

Show index stats plus lightweight helpers:

- heuristic architecture checks (import layering hints)
- git-aware change summary (local working tree vs `HEAD`)

```bash
linkmap stats
```

### `linkmap explain <symbol | path>`

Explain a symbol’s relationships and warnings, or explain a path query:

```bash
linkmap explain generate_html
linkmap explain "A -> B"
```

## Supported languages (v1)

Indexing is Tree-sitter + queries + heuristics. Currently supported:

- JavaScript / TypeScript / TSX
- Python
- Go
- Rust
- PHP (including Laravel route helper)

## Troubleshooting

### “index format is missing or out of date”

Run:

```bash
linkmap init
```

### The viewer opens but looks empty

Try `Fit` in the toolbar, or regenerate after re-indexing:

```bash
linkmap init
linkmap show <symbol>
```

### Routes aren’t showing

Route extraction is best-effort and framework-specific. If you find a pattern Linkmap misses, open an issue and include a minimal example.

## Safety and scope

Linkmap is **best-effort static analysis**, not a type checker or runtime tracer:

- no type inference
- no dynamic dispatch tracing
- conservative matching (skips ambiguous edges)

It is designed for local use and writes only under `.link/`.

