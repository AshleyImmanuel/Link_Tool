## Contributing

### Development setup

- Install Rust (stable) via [rustup.rs](https://rustup.rs/).
- Build:

```bash
cargo build
```

### Quality gates

Before opening a PR, run:

```bash
cargo fmt
cargo test
cargo clippy -- -D warnings
```

### Project principles

- Keep Link local-first and offline-first by default.
- Prefer deterministic, best-effort static analysis over “invented” edges.
- Avoid background daemons and servers in the core CLI.
- Keep language integrations small: definitions, calls, imports, and minimal stack adapters.

### Adding a language

- Add a Tree-sitter grammar dependency in `Cargo.toml`.
- Extend `src/lang.rs` and `src/parser.rs`.
- Add a query file under `queries/<language>.scm`.
- Add an integration test in `tests/integration.rs`.

