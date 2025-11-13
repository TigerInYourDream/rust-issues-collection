# Async Resource Drop Trap - Correct Example

## The Solution

This example demonstrates the **correct pattern** for handling async resource cleanup.

## Key Patterns

### 1. Explicit Async Shutdown Method

```rust
async fn shutdown(self) {
    // 1. Signal task to stop
    self.shutdown_tx.send(true).ok();

    // 2. Wait for cleanup to complete
    self.shutdown_complete.notified().await;

    // 3. Join task handle
    self.task_handle.await.ok();
}
```

### 2. Graceful Shutdown with Timeout

```rust
async fn shutdown_with_timeout(self, timeout: Duration) -> Result<(), &'static str> {
    self.shutdown_tx.send(true).ok();

    tokio::select! {
        _ = self.shutdown_complete.notified() => Ok(()),
        _ = tokio::time::sleep(timeout) => {
            self.task_handle.abort();
            Err("Shutdown timed out")
        }
    }
}
```

### 3. Drop as Safety Net (Not Primary Cleanup)

```rust
impl Drop for BackgroundWorker {
    fn drop(&mut self) {
        if !self.task_handle.is_finished() {
            eprintln!("WARNING: Dropped without shutdown()!");
            self.task_handle.abort();
        }
    }
}
```

## Running the Example

```bash
cargo run
```

**Expected Output:**
You'll see three examples:
1. Graceful shutdown - all cleanup runs
2. Shutdown with timeout - handles slow cleanup
3. Drop without shutdown - shows safety net warning

## Running Tests

```bash
cargo test
```

All tests should pass, demonstrating:
- Graceful shutdown completes cleanup
- Timeout handling works correctly
- Natural task completion is handled properly

## Benefits

✅ **Explicit control** - caller decides when cleanup happens
✅ **Async-friendly** - no blocking in Drop
✅ **Timeout handling** - prevents hanging on shutdown
✅ **Safety net** - Drop warns if cleanup was skipped
✅ **Testable** - easy to verify cleanup behavior

## Environment

- **Rust Version**: 1.85.0 (stable)
- **Toolchain**: aarch64-apple-darwin
- **OS**: macOS 15.1.1

## Real-World Applications

This pattern is essential for:
- **Database connections** - commit transactions before closing
- **Network services** - graceful connection shutdown
- **File operations** - flush buffers and sync to disk
- **Background workers** - complete in-flight work
- **Resource pools** - drain and cleanup properly

## Related Patterns

- **CancellationToken pattern** (tokio-util)
- **Graceful shutdown** in web servers
- **Drop guard types** for automatic cleanup
- **RAII with explicit cleanup** pattern
