# LinkTool: The Ultimate Code Visualization & Static Analysis CLI

> **Git for understanding code.** Instantly generate interactive, architectural code maps from your source code using local-only, best-effort static analysis.

LinkTool is an insanely fast, local-first CLI designed for developers who need to explore complex codebases, map out legacy architecture, and visually navigate dependencies without the bloat of cloud servers or heavy IDE plugins.

Powered by **Tree-sitter** and **SQLite**, LinkTool extracts symbols, imports, and function calls, and renders a stunning, interactive HTML codemap visualization directly in your browser.

## Why LinkTool? 

If you've ever struggled to build a mental map of a massive repository, LinkTool solves the "spaghetti code" problem.

- 🔒 **100% Local & Offline:** No servers, no background daemons, no data leaves your machine. Perfect for enterprise and proprietary codebases.
- ⚡ **Lightning Fast:** Analyzes thousands of files in seconds using Tree-sitter.
- 🗺️ **Interactive Code Maps:** Generates a dynamic, glassmorphism HTML graph visualization.
- 📁 **Hierarchical Module Collapsing:** Avoid the "spaghetti graph" problem by clustering nodes by their directory path for high-level architectural views.
- 🎯 **Smart Edge Filtering:** Instantly hide noise (variables, imports) to focus entirely on the "happy path" of major components.
- 🛠️ **IDE Integrated:** Double-click any node in the graph to instantly jump to the exact file and line in VS Code!

## Supported Languages

LinkTool uses Tree-sitter for robust parsing across modern stacks:
- JavaScript / TypeScript (incl. JSX/TSX)
- Python
- Go
- Rust
- Java **[NEW!]**
- PHP

*(LinkTool also understands framework-specific paradigms out-of-the-box, such as Express routes and Laravel controllers!)*

## Quick Start Guide

### Installation

LinkTool is currently distributed as a lightweight, precompiled Rust binary.

**Windows (PowerShell):**
```powershell
powershell -NoProfile -ExecutionPolicy Bypass -Command "iwr -useb https://raw.githubusercontent.com/AshleyImmanuel/Link_Tool/main/install.ps1 | iex"
```

**macOS/Linux (Curl):**
```bash
curl -fsSL https://raw.githubusercontent.com/AshleyImmanuel/Link_Tool/main/install.sh | sh
```

**Build from Source (Cargo):**
```bash
cargo install --path .
```

### Usage

Run LinkTool from the root of the repository you want to inspect:

```bash
# 1. Scan and index the codebase (creates a lightweight .link/index.db)
linkmap init

# 2. Search for a specific symbol or function
linkmap search AuthController

# 3. Generate and open the interactive Code Map in your browser
linkmap show AuthController

# 4. [NEW!] Export the dependency graph to standard Graphviz DOT format
linkmap export AuthController --dot > graph.dot

# 5. Incrementally update the index after changing your code
linkmap update
```

## Security & Privacy 

LinkTool has undergone rigorous security auditing to ensure safe execution on untrusted repositories:
- **No Remote Execution:** Completely offline and local.
- **XSS & Injection Hardened:** The HTML viewer and SQLite database are fortified against payload injection.
- **Path Traversal Protection:** Safely sandboxed to the `.link/` directory.

## Contributing

LinkTool is an open-source project and we welcome contributions! Want to add a new language parser? It takes less than 5 minutes using our new `define_languages!` macro.

Please see our [CONTRIBUTING.md](CONTRIBUTING.md) for details on how to get started, and review our [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
