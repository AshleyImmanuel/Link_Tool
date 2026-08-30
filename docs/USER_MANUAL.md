# Linkmap user manual

This is a practical guide for new users.

Linkmap helps you answer questions like:

- “Where is this thing defined?”
- “What calls this function / uses this component?”
- “If I change X, what else might be impacted?”

It does this by building a **local index** of your repo, then showing a **graph view** (plus search, stats, and snapshots).

## Before you start (1 minute)

- Run Linkmap **from the root of the repo** you want to analyze.
- Linkmap writes a local cache under **`.link/`** (inside your repo).
- It’s intentionally best‑effort: it may miss some links rather than guess.

## Quick start (most common workflow)

```bash
linkmap init
linkmap search <name>
linkmap show <name>
```

Example:

```bash
linkmap init
linkmap search HeroSection
linkmap show HeroSection
```

After you edit code later:

```bash
linkmap update
```

## What Linkmap indexes (plain English)

Linkmap stores:

- **Symbols**: “things in code” (functions, classes, components, routes, handlers)
- **Edges**: “relationships” (calls, imports, renders, route → handler)

In the viewer you’ll see dots/colors for different symbol kinds (component/route/function/call/etc.).

## Commands (with when to use them)

### `linkmap init` (first run, or when it complains)

Use this when:

- You run Linkmap in a repo for the first time
- You see: “index format is missing or out of date”

```bash
linkmap init
```

What it does:

- Scans the repo and builds `.link/index.db`
- Skips common build/vendor folders
- Skips very large files for safety/performance

### `linkmap update` (after code changes)

Use this after you change code and want the index updated quickly:

```bash
linkmap update
```

### `linkmap search <query>` (find the right symbol)

Use this when you don’t remember the exact name or there are multiple matches:

```bash
linkmap search router
linkmap search GET
```

### `linkmap show <symbol>` (open the graph)

Use this when you want to *see connections* (what calls/imports/renders what):

```bash
linkmap show generate_html
```

Options:

- `--json`: print the graph JSON instead of opening the browser

Viewer basics:

- Use **Fit** to zoom out and see everything
- Use the **search box** to jump to a node
- Double‑click a node to open it in VS Code (via `vscode://file/...`)

### `linkmap list` (browse indexed definitions)

```bash
linkmap list
```

### `linkmap snapshot` (save a “structure snapshot”)

Use this when you want to save/share the current architecture state:

```bash
linkmap snapshot
linkmap snapshot --out before.json
```

### `linkmap diff <from> <to>` (compare snapshots)

Use this to see structural changes between two snapshots:

```bash
linkmap diff before.json after.json
```

Options:

- `--json`: machine‑readable output

### `linkmap history` (see what you ran)

```bash
linkmap history
linkmap history --all
```

### `linkmap stats` (repo health + change summary)

Use this to get a quick overview:

```bash
linkmap stats
```

It includes:

- Index counts (files/symbols/edges)
- Lightweight architecture hints (heuristics)
- Git-aware change summary (working tree vs `HEAD`)

### `linkmap explain <query>` (text explanation)

Use this when you want a readable explanation instead of a graph:

```bash
linkmap explain SomeSymbol
linkmap explain "A -> B"
```

## Supported languages (v1)

- JavaScript / TypeScript / TSX
- Python
- Go
- Rust
- PHP

Extra helpers (best-effort):

- Express routes (Node): `app.get("/path", handler)` → route nodes like `GET /path`
- Laravel routes (PHP): `Route::get("/path", "Controller@method")` → route nodes like `GET /path`

## Troubleshooting

### “index format is missing or out of date”

```bash
linkmap init
```

### The viewer opens but looks empty / no nodes

- Click **Fit**
- If it still looks wrong, rebuild the index:

```bash
linkmap init
linkmap show <symbol>
```

### Routes aren’t detected

Route support is pattern-based. If your framework uses a different style, Linkmap may miss it. If you report it, include a small code snippet that shows the route style.

## Safety and limits (important)

Linkmap is **not** a type-checker or runtime tracer. It won’t understand every dynamic pattern.

What it *won’t* do:

- type inference
- runtime dispatch tracing
- “eval/reflection” understanding

What it *will* do:

- stay local/offline
- write only under `.link/`
- prefer “no edge” over a wrong edge

