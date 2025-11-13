# Repository Guidelines

## Project Structure & Module Organization
The primary runnable crate lives in `rust-logout-issue/`. Within it, `src/problem.rs` reproduces the Tokio shutdown panic, `src/solution.rs` houses the state-machine fix (and embeds unit tests), and `src/main.rs` guides readers toward the two binaries. Place new reproductions or writeups under `submissions/<issue-name>/`; the zipped snapshots there are historical references and should stay untouched unless you intentionally regenerate them. Temporary build artifacts accrue in `target/`; keep that directory out of versioned contributions.

## Build, Test, and Development Commands
Run `cargo check` inside `rust-logout-issue/` for a fast compile-time sanity pass. Format with `cargo fmt --all`, and lint using `cargo clippy -- -D warnings` to keep async patterns tight. Use `cargo run --bin problem` to demonstrate the failing workflow and `cargo run --bin solution` to showcase the state-machine fix. End-to-end and regression suites run with `cargo test` (add `-- --nocapture` when you need full state-machine logs). If you introduce new binaries, mirror the existing pattern in `Cargo.toml`.

## Coding Style & Naming Conventions
The crate targets Rust 2021 with 4-space indentation and `rustfmt` defaults. Prefer `snake_case` for functions and modules, `UpperCamelCase` for types and states, and keep error messages actionable. Async scenarios should log via `log`/`env_logger`; reserve `println!` for CLI feedback only. Document non-obvious flows with rustdoc comments, as seen in `src/solution.rs`, rather than inline prose.

## Testing Guidelines
Leverage `#[tokio::test]` for asynchronous cases and favor deterministic timers via `tokio::time::pause` or bounded sleeps. Store integration tests under `rust-logout-issue/tests/` when they exercise binaries, and inline unit tests next to the state machine logic. Every bug reproduction needs a regression test that fails before your fix and passes afterward. Capture both the "problem" and "solution" behaviors to illustrate why a change is necessary.

## Commit & Pull Request Guidelines
This archive ships without git history, so adopt a slim Conventional Commits style—`fix: describe state-machine timeout`—written in present tense and under 72 characters. Commits should bundle one logical change alongside the corresponding `cargo fmt`/`cargo clippy` runs. PRs must link to the reproduced issue, summarize test coverage, and include relevant runtime logs (for example `env RUST_LOG=info cargo run --bin solution`). Screenshot CLI output when it clarifies the behavior you are documenting.
