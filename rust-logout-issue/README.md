# Tokio Runtime Shutdown Issue with Async Tasks

## Problem Summary

This project demonstrates a critical issue that occurs when shutting down a tokio runtime while async tasks (particularly deadpool-runtime tasks from the Matrix Rust SDK) are still running, and presents a state machine-based solution.

## Environment Information

- **Rust Version**: 1.90.0 (1159e78c4 2025-09-14)
- **Cargo Version**: 1.90.0 (840b83a10 2025-07-30)
- **Toolchain**: stable-aarch64-apple-darwin
- **Target Triple**: aarch64-apple-darwin
- **Operating System**: macOS 26.1 (Build 25B5057f)
- **Architecture**: Apple Silicon (ARM64)

## Issue Background

In the Robrix Matrix client (built with Makepad), the logout process involves:
1. Stopping the sync service
2. Performing server-side logout
3. Cleaning up the Matrix SDK client
4. Shutting down the tokio runtime
5. Restarting a new runtime for future logins

The Matrix SDK internally uses `rusqlite` for data storage, which uses `deadpool-sqlite` as a connection pool, which in turn depends on `deadpool-runtime` for managing async tasks.

### The Problem

When `CLIENT.lock().unwrap().take()` is called (dropping the Matrix client) and the tokio runtime is immediately shut down via `shutdown_background()`, the following panic occurs:

```
thread 'main' panicked at deadpool-runtime-0.1.4/src/lib.rs:101:22:
there is no reactor running, must be called from the context of a Tokio 1.x runtime
```

**Dependency chain:**
```
matrix-sdk -> matrix-sdk-sqlite -> rusqlite -> deadpool-sqlite -> deadpool-runtime
```

**Root cause:** `shutdown_background()` does NOT wait for async tasks to complete. When the client is dropped, its internal deadpool tasks are still running and attempt to execute after the runtime has been closed.

## The Solution

Implement a **logout state machine** that ensures proper ordering and timing:

```
Idle → PreChecking → StoppingSyncService → LoggingOutFromServer
  → PointOfNoReturn → CleaningAppState → ShuttingDownTasks
  → RestartingRuntime → Completed
```

**Key improvement:** After dropping the client in `CleaningAppState`, we **wait for cleanup confirmation** using a oneshot channel with timeout. This wait period gives async tasks sufficient time to complete properly before we shutdown the runtime.

## Project Structure

```
rust-logout-issue/
├── Cargo.toml              # Project configuration
├── README.md               # This file
├── src/
│   ├── main.rs            # Entry point with usage instructions
│   ├── problem.rs         # Demonstrates the problematic approach
│   └── solution.rs        # Demonstrates the state machine solution
└── tests/
    └── (integration tests included in solution.rs)
```

## Usage

### Run the problematic approach

```bash
cargo run --bin problem
```

This demonstrates what happens when you shutdown the runtime immediately after dropping the client. In a real scenario with deadpool-runtime, this would cause a panic.

### Run the solution

```bash
cargo run --bin solution
```

This demonstrates the state machine solution that prevents the panic by ensuring async tasks have time to complete.

### Run tests

```bash
cargo test
```

All tests should pass, demonstrating that the solution approach is stable.

### Run code quality checks

```bash
cargo check
cargo clippy
```

## Key Learnings

1. **tokio's `shutdown_background()` doesn't wait for tasks** - This is the root cause of the race condition
2. **Complex async resources need time to destruct** - Matrix SDK and similar libraries with internal connection pools require proper cleanup time
3. **State machines provide clear ordering guarantees** - Breaking down the logout process into discrete states with explicit transitions prevents timing issues
4. **tokio-console is invaluable for debugging** - Use it to observe async task lifecycles and identify when tasks are still running

## Debugging Method

The original issue was identified using:
1. **tokio-console** to monitor async task lifecycles
2. Adding delays between `shutdown_background()` and runtime restart
3. Observing that deadpool-related tasks were still present after shutdown

## Alternative Approaches Attempted

1. **Looking for Matrix SDK cleanup APIs** - No public API exists to manually cleanup deadpool-runtime
2. **Using `mem::forget` to leak resources during app shutdown** - Works but is overly aggressive
3. **Adjusting destruction order** - Helps but doesn't fully solve the race condition

## Solution Benefits

- ✅ No more deadpool-runtime panics
- ✅ Clear, traceable logout flow with progress reporting
- ✅ Proper error handling at each state
- ✅ Point of no return semantics for better UX
- ✅ Testable and maintainable code

## License

MIT License

Copyright (c) 2025

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

## References

- Original issue discussion: https://github.com/project-robius/robrix/pull/432#discussion_r2171202111
- Tokio runtime shutdown documentation
- deadpool-runtime crate documentation
- Matrix Rust SDK source code

## Contributing

This is a demonstration project for educational purposes. If you encounter similar issues in your projects, consider implementing a state machine approach to manage complex async resource lifecycles.
