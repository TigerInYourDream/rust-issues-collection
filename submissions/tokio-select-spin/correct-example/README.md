# Busy Loop Fix (Correct)

This crate shows how to restructure the `tokio::select!` loop so the idle branch
cooperates with Tokio's scheduler. The broken variant used an immediately-ready
branch (mimicking `sleep(Duration::ZERO)`), starving the real work. Here we
reuse a `tokio::time::Interval`, so each idle tick introduces a real delay and
the consumer remains responsive.

## Requirements

- Rust 1.70+
- Tokio 1.47.1

## Running the Demo

```bash
cargo run --release
```

The program prints the number of idle ticks that occurred; it should remain well
below the guard threshold thanks to the interval-based backoff. Run `cargo test`
to ensure the regression check passes.

## Key Files

- `src/main.rs` — contains the cooperative `tokio::select!` loop and the fix.
