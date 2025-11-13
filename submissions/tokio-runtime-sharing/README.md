# Tokio Runtime Sharing Pattern: Arc vs Mutex Dilemma

## Problem Overview

A fundamental challenge in Rust async programming: how to share a global Tokio runtime across multiple components while maintaining the ability to controllably shutdown and restart it. This issue commonly arises in GUI applications that need to handle logout/login flows.

## The Core Dilemma

When sharing a Tokio runtime globally, developers face an impossible choice between two flawed approaches:

### 1. Arc\<Runtime\> Approach ❌
**Pros:**
- Easy to share via Arc cloning
- No lock contention
- Can hold references for extended periods

**Cons:**
- **Cannot force shutdown** - no way to ensure all Arc clones are dropped
- No lifecycle control - runtime persists as long as any clone exists
- Risk of dangling references after attempted shutdown
- May cause "no reactor running" panics when runtime is gone but tasks remain

### 2. Mutex\<Option\<Runtime\>\> Approach ❌
**Pros:**
- Can force shutdown via `take()`
- Full lifecycle control

**Cons:**
- **MutexGuard is not Send** - cannot hold across `.await` points
- Must repeatedly lock/unlock for continuous work
- Performance overhead from constant locking
- Lock contention risks with multiple accessors
- Deadlock potential in complex scenarios

## Real-World Context

This issue was discovered in the Robrix Matrix client (a Makepad-based GUI application) where:
- Time-series processing (TSP) workers need long-lived runtime access
- Event handlers spawn async tasks
- Logout flow requires clean runtime shutdown
- Login flow needs fresh runtime startup
- Matrix SDK uses internal connection pools (rusqlite, deadpool) with async cleanup

## The Solution: Arc + CancellationToken Pattern ✅

Combine the best of both worlds by separating concerns:
- **Arc\<Runtime\>** for easy sharing (solving ergonomics)
- **CancellationToken** for shutdown coordination (solving control)
- **Graceful shutdown sequence** to prevent panics

```rust
static TOKIO_RUNTIME: OnceLock<Arc<Runtime>> = OnceLock::new();
static SHUTDOWN_TOKEN: OnceLock<CancellationToken> = OnceLock::new();

// Components freely clone Arc
fn get_runtime() -> Arc<Runtime> {
    Arc::clone(TOKIO_RUNTIME.get().unwrap())
}

// Tasks listen for cancellation
loop {
    tokio::select! {
        _ = shutdown_token.cancelled() => break,
        _ = do_work() => { /* ... */ }
    }
}

// Graceful shutdown
async fn graceful_shutdown() {
    shutdown_token.cancel();            // 1. Signal all tasks
    wait_for_tasks_to_complete().await;  // 2. Wait for cleanup
    // 3. Now safe to shutdown runtime
}
```

## Key Benefits

This pattern provides:
- ✅ **Easy sharing** - Arc cloning without restrictions
- ✅ **No lock contention** - no Mutex overhead
- ✅ **Long-lived references** - TSP workers can hold Arc indefinitely
- ✅ **Controlled shutdown** - CancellationToken coordinates cleanup
- ✅ **Prevents panics** - avoids "no reactor running" errors
- ✅ **Clean restart** - can safely create new runtime after shutdown

## Project Structure

```
tokio-runtime-sharing/
├── broken-example/       # Demonstrates the problems
│   ├── src/
│   │   ├── main.rs      # Entry point
│   │   ├── arc_approach.rs    # Shows Arc limitations
│   │   └── mutex_approach.rs  # Shows Mutex difficulties
│   └── Cargo.toml
├── correct-example/      # Demonstrates the solution
│   ├── src/
│   │   └── main.rs      # Arc + CancellationToken implementation
│   └── Cargo.toml
└── README.md            # This file
```

## Running the Examples

### See the Problems
```bash
cd broken-example
cargo run --bin arc_problem     # Shows Arc shutdown issues
cargo run --bin mutex_problem   # Shows Mutex ergonomics issues
```

### See the Solution
```bash
cd correct-example
cargo run                       # Shows working Arc + CancellationToken pattern
cargo test                      # Verify the solution works
```

## Technical Details

### Why Arc Alone Fails

Arc provides shared ownership but no control over when the last reference is dropped. In production systems with background tasks (TSP workers, event handlers, connection pools), you cannot guarantee all Arc clones will be dropped when needed.

### Why Mutex Alone Fails

MutexGuard is deliberately not Send to prevent data races. This means you cannot hold a guard across an `.await` point, forcing awkward patterns like:
```rust
// Cannot do this:
let guard = RUNTIME.lock().unwrap();
guard.spawn(async { ... }).await;  // Error: guard is not Send

// Must do this instead:
{
    let guard = RUNTIME.lock().unwrap();
    guard.spawn(async { ... });
}  // Drop guard before await
```

### The CancellationToken Solution

CancellationToken provides a broadcast signaling mechanism that's:
- Clone-able and Send
- Zero-cost when not cancelled
- Integrates with tokio::select! for responsive cancellation
- Allows coordinated shutdown across all components

## Common Pitfalls

1. **Forgetting to wait after cancellation** - Always allow time for async cleanup tasks to complete
2. **Not using select! in long-running tasks** - Tasks must actively check for cancellation
3. **Mixing shutdown patterns** - Use CancellationToken consistently across all components
4. **Ignoring connection pool cleanup** - Libraries like deadpool need time for async cleanup

## Related Issues

This solution builds upon the state machine pattern from `rust-logout-issue` which handles the logout sequence timing. Together they provide:
1. Proper logout sequencing (state machine)
2. Runtime sharing and lifecycle (Arc + CancellationToken)

## Environment

- Rust Version: 1.90.0 (stable)
- Target: aarch64-apple-darwin
- Dependencies:
  - tokio 1.x with full features
  - tokio-util for CancellationToken
  - log/env_logger for debugging
  - anyhow for error handling

## Quality Standards

All code examples pass:
- `cargo check` - No compilation errors
- `cargo test` - All tests pass
- `cargo clippy` - No linting warnings

## Educational Value

This issue teaches:
- Fundamental trade-offs in Rust's type system
- Real-world async patterns beyond basic tutorials
- How to think about shared state in production systems
- Separation of concerns in system design
- Why obvious solutions don't always work in Rust

## License

MIT

## Credits

Issue discovered and solution developed for the Robrix Matrix client project.
Pattern refined through production usage with time-series processing workloads.