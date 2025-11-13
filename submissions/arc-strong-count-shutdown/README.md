# Arc Strong Count Leak Blocks Logout Cleanup

This submission documents a real-world shutdown failure we hit while building the Robrix Matrix client. The app keeps a background Matrix SDK client alive across the GUI, and we expected `Drop` to abort its async helpers during logout. However, those helpers each held a strong `Arc<ClientInner>` clone. The last user-facing `Arc` was dropped, but the inner state never reached `Drop`, so the cleanup confirmation channel never fired. Our logout state machine waited for that confirmation before shutting down the global Tokio runtime, so the whole flow stalled and, once we force-killed the runtime, `deadpool-runtime` panicked because outstanding tasks were still polling a reactor that had gone away.

The fix was to make background tasks hold `Weak<ClientInner>` handles. They upgrade the `Weak` at the start of every loop iteration, do their work, then release the temporary `Arc` before the next `await`. Once logout drops the last strong `Arc`, subsequent `upgrade` calls fail, every task exits gracefully, and `ClientInner::drop` runs immediately to notify the state machine that it is safe to proceed.

## Layout

```
arc-strong-count-shutdown/
├── broken-example/         # Tasks keep strong Arc clones; logout never finishes
└── correct-example/        # Tasks downgrade to Weak and exit when client drops
```

## How to Reproduce

```bash
# Broken flow: times out waiting for ClientInner::drop
cd submissions/arc-strong-count-shutdown/broken-example
cargo run

# Correct flow: receives cleanup signal in time
cd ../correct-example
cargo run
```

Both crates include regression tests:

```bash
cargo test                      # Asserts the expected failure/success respectively
cargo clippy -- -D warnings     # Lints must stay clean
```

## Environment

- Rust: 1.90.0 (stable-aarch64-apple-darwin)
- Target triple: aarch64-apple-darwin
- OS: macOS 15.1 (25B5057f)
- Architecture: Apple Silicon (ARM64)
- License: MIT

## Key Takeaways

1. Dropping the GUI-visible `Arc` is not enough; background tasks can silently keep `ClientInner` alive forever.
2. Waiting for a oneshot confirmation is a great guardrail, but it only works when the drop handler actually runs.
3. `Weak` plus a well-defined shutdown contract lets long-lived `tokio::spawn` tasks exit without needing `AbortHandle`s or global registries.
4. The fix integrates cleanly with the existing logout state machine—once the drop signal arrives, we can confidently proceed to `Runtime::shutdown_background()` without triggering deadpool panics.
