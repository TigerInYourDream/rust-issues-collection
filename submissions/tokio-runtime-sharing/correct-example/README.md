# Correct Solution: Arc<Runtime> + CancellationToken

This example demonstrates the CORRECT approach for sharing a Tokio Runtime with controlled shutdown capability.

## The Solution

Combine `Arc<Runtime>` with `CancellationToken` to get the best of both worlds:

```rust
static TOKIO_RUNTIME: OnceLock<Arc<Runtime>> = OnceLock::new();
static SHUTDOWN_TOKEN: OnceLock<CancellationToken> = OnceLock::new();
```

## How It Works

### 1. Easy Sharing with Arc

Components can clone the `Arc<Runtime>` and use it freely:

```rust
struct TspWorker {
    runtime: Arc<Runtime>,
    shutdown_token: CancellationToken,
}

fn new() -> Self {
    Self {
        runtime: get_runtime(),  // Clone the Arc
        shutdown_token: get_shutdown_token(),
    }
}
```

### 2. Controlled Shutdown with CancellationToken

When shutdown is needed:

```rust
async fn graceful_shutdown(task_handles: Vec<JoinHandle<()>>) -> Result<()> {
    // Step 1: Broadcast shutdown signal to all tasks
    get_shutdown_token().cancel();

    // Step 2: Wait for all tasks to complete cleanup
    for handle in task_handles {
        handle.await?;
    }

    // Step 3: Additional cleanup wait period
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Step 4: Now safe to shutdown runtime
    // All Arc references can still exist, but tasks are stopped
}
```

### 3. Tasks Respect Shutdown Signal

All long-running tasks use `tokio::select!` to listen for cancellation:

```rust
loop {
    tokio::select! {
        _ = shutdown_token.cancelled() => {
            log::info!("Task received shutdown signal, cleaning up");
            break;
        }
        _ = tokio::time::sleep(Duration::from_millis(500)) => {
            // Do work
        }
    }
}
```

## Key Benefits

| Feature | Arc Only | Mutex Only | **Arc + CancellationToken** |
|---------|----------|------------|--------------------------|
| Easy sharing | ✅ | ❌ | ✅ |
| Long-lived references | ✅ | ❌ | ✅ |
| No lock contention | ✅ | ❌ | ✅ |
| Controlled shutdown | ❌ | ✅ | ✅ |
| Graceful cleanup | ❌ | ❌ | ✅ |
| Prevents deadpool panic | ❌ | ⚠️ | ✅ |

## Real-World Application

This pattern is essential for applications like the Robrix Matrix client where:

1. **TSP (Time-Series Processing)** needs long-lived runtime access for background data processing
2. **Logout flow** requires shutting down the Matrix SDK client and restarting the runtime
3. **Multiple components** (sync service, event handlers, UI updates) all need runtime access

The solution prevents the `deadpool-runtime` panic by ensuring:
- All async tasks receive shutdown signal
- Tasks have time to complete cleanup
- Runtime is only shutdown after confirmation that tasks are done

## Environment Information

- **Rust Version**: 1.90.0 (stable)
- **Toolchain**: aarch64-apple-darwin
- **Operating System**: macOS 15.1
- **Architecture**: Apple Silicon (ARM64)

## How to Run

```bash
# Run the correct example
cargo run

# Run tests
cargo test

# Check compilation
cargo check

# Run linter
cargo clippy
```

## Output

You should see:
1. TSP worker starting tasks with Arc references
2. Continuous work without repeated locking
3. Graceful shutdown sequence with all tasks completing cleanly
4. No panics, no deadlocks

## Related Issues

This solution addresses the same underlying problem as the `rust-logout-issue` example:
- Preventing premature runtime shutdown while tasks are still running
- Avoiding the "there is no reactor running" panic from deadpool-runtime
- Implementing a clean shutdown sequence

## License

MIT
