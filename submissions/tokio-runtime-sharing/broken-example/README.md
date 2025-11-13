# Broken Example: Tokio Runtime Sharing Problems

This example demonstrates the INCORRECT approaches for sharing a Tokio Runtime across multiple components.

## The Problem

In applications with a global Tokio runtime that needs to be:
1. Shared across multiple components (e.g., time-series processing workers)
2. Controllably shutdown and restarted (e.g., during logout/login flows)

There are two common but problematic approaches:

### Approach 1: Arc<Runtime>

```rust
static TOKIO_RUNTIME: OnceLock<Arc<Runtime>> = OnceLock::new();
```

**Pros:**
- ✅ Easy to share: Components can clone the Arc and use it freely
- ✅ No lock contention
- ✅ Can hold long-lived references

**Cons:**
- ❌ Cannot force all Arc clones to drop
- ❌ No way to ensure timely shutdown
- ❌ Risk of dangling references after attempted shutdown
- ❌ Cannot guarantee runtime is actually closed when needed

### Approach 2: Mutex<Option<Runtime>>

```rust
static TOKIO_RUNTIME: Mutex<Option<Runtime>> = Mutex::new(None);
```

**Pros:**
- ✅ Allows controlled shutdown via `take()`
- ✅ Ensures unique access at any time

**Cons:**
- ❌ Cannot hold long-lived references (MutexGuard is not `Send`)
- ❌ Must lock/unlock repeatedly for continuous work
- ❌ Lock contention between components
- ❌ Risk of deadlocks
- ❌ Complex and error-prone to use correctly

## Environment Information

- **Rust Version**: 1.90.0 (stable)
- **Toolchain**: aarch64-apple-darwin
- **Operating System**: macOS 15.1
- **Architecture**: Apple Silicon (ARM64)

## How to Run

```bash
# Run the broken examples
cargo run

# Check compilation
cargo check

# Run linter
cargo clippy
```

## What You'll See

The output demonstrates:
1. Arc approach: High reference count, no way to force shutdown
2. Mutex approach: Repeated locking, cannot keep long-lived references

## The Fix

See the `correct-example` directory for the proper solution using Arc + CancellationToken.

## License

MIT
