# Async Resource Drop Trap - Broken Example

## The Problem

This example demonstrates what goes **wrong** when handling async resource cleanup in the synchronous `Drop` trait.

## Issues Demonstrated

1. **`abort()` kills the task immediately** - cleanup code never runs
2. **Cannot `.await` in `Drop`** - `async fn drop()` doesn't exist
3. **`block_on` in `Drop` can deadlock** - especially when called from async context
4. **Manual cleanup is unreliable** - resources may still be in use by the aborted task

## Running the Example

```bash
cargo run
```

**Expected Output:**
You'll see the task being aborted mid-execution, with its cleanup code never running.

## Environment

- **Rust Version**: 1.85.0 (stable)
- **Toolchain**: aarch64-apple-darwin
- **OS**: macOS 15.1.1

## Why This Matters

In production applications, this pattern leads to:
- Resource leaks (database connections, file handles)
- Data corruption (partial writes not flushed)
- Inconsistent state (cleanup transactions not committed)
- Hard-to-debug issues (cleanup failures are silent)

See the `correct-example` for the proper solution using explicit async cleanup.
