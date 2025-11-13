# Busy Loop Reproduction (Broken)

This crate demonstrates how `tokio::select!` can devolve into a hot loop when it
pairs an `mpsc::UnboundedReceiver` with an immediately-ready branch (a common
anti-pattern that mimics `tokio::time::sleep(Duration::ZERO)`). The consumer
attempts to throttle itself, but the zero-delay branch is always ready, so the
loop keeps re-scheduling instantly and consumes a full CPU core.

## Requirements

- Rust 1.70+
- Tokio 1.47.1

## Running the Demo

```bash
cargo run --release
```

Watch the logs for the warning that the spin limit is hit. You can also run
`cargo test` to assert that the reproduction actually spins.

## Key Files

- `src/main.rs` — contains the `tokio::select!` loop that triggers the busy spin.
