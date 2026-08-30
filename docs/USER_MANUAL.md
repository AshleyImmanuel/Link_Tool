# Linkmap user manual

This is a practical guide for new users.

Linkmap helps you answer questions like:

- “Where is this thing defined?”
- “What calls this function / uses this component?”
- “If I change X, what else might be impacted?”

It does this by making a **local map** of your project, then showing a **graph** (a picture of how things connect).

## Before you start (1 minute)

- Run Linkmap **from the root of the repo** you want to analyze.
- Linkmap saves its local data in a folder named **`.link/`** (inside your repo).
- It tries to be safe: if it’s not sure about a connection, it may skip it instead of guessing.

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

## What Linkmap “understands” (plain English)

Linkmap stores:

- **Things**: functions, classes, components, routes, handlers
- **Connections**: “this calls that”, “this imports that”, “this route goes to that handler”, etc.

In the viewer you’ll see dots/colors for different types (component/route/function/call/etc.).

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

### `linkmap search <query>` (find the right thing)

Use this when you don’t remember the exact name or there are multiple matches:

```bash
linkmap search router
linkmap search GET
```

### `linkmap show <name>` (open the graph)

Use this when you want to *see connections* (what calls/imports/renders what):

```bash
linkmap show generate_html
```

Options:

- `--json`: print the graph JSON instead of opening the browser

Viewer basics:

- Use **Fit** to zoom out and see everything
- Use the **search box** to jump to a box in the graph
- Double‑click a box to open it in VS Code

### `linkmap list` (browse what Linkmap found)

```bash
linkmap list
```

### `linkmap snapshot` (save a “project snapshot”)

Use this when you want to save/share the current “shape” of the project:

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

### `linkmap stats` (quick overview)

Use this to get a quick overview:

```bash
linkmap stats
```

It includes:

- Counts (how many files / things / connections were found)
- Simple architecture hints
- Git-aware change summary (working tree vs `HEAD`)

### `linkmap explain <query>` (simple text explanation)

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

Linkmap is **not perfect** and it doesn’t run your code. It won’t understand every dynamic trick.

What it *won’t* do:

- type inference
- runtime dispatch tracing
- “eval/reflection” understanding

What it *will* do:

- stay local/offline
- write only under `.link/`
- prefer “no edge” over a wrong edge

