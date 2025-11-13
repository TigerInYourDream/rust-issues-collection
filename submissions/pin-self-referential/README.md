# Pin and Self-Referential Structures - Complete Example

This is a complete, production-ready example demonstrating self-referential structures in Rust and the correct use of Pin.

## What's Included

### 1. Broken Example (`broken-example/`)
Demonstrates the problems and undefined behavior:
- Simple self-referential struct with raw pointers
- Vec reallocation breaking internal pointers
- Memory layout analysis
- Why lifetimes cannot solve this

### 2. Correct Example (`correct-example/`)
Shows the proper solutions:
- Pin basics and concepts
- Alternative designs (avoiding self-reference)
- Complete async buffer reader with pin_project
- Full test coverage

### 3. Submission Document (`SUBMISSION.md`)
Complete questionnaire answers for Corust.ai data annotation project.

## Quick Start

### Run Broken Example
```bash
cd broken-example

# Run examples (may produce UB)
cargo run

# Detect UB with Miri
cargo +nightly miri run
```

### Run Correct Example
```bash
cd correct-example

# Run examples
cargo run

# Run all tests
cargo test

# Lint with clippy
cargo clippy

# Verify no UB with Miri
cargo +nightly miri test
```

## Verification Status

All projects pass quality checks:

**Broken Example:**
- cargo check: PASS
- cargo run: PASS (but exhibits UB)
- cargo +nightly miri run: DETECTS UB (as expected)

**Correct Example:**
- cargo check: PASS
- cargo test: PASS (9/9 tests)
- cargo clippy: PASS
- cargo +nightly miri test: PASS (no UB detected)

## File Structure

```
pin-self-referential/
├── README.md                      # This file
├── SUBMISSION.md                  # Complete questionnaire answers
├── broken-example/
│   ├── Cargo.toml
│   ├── README.md
│   └── src/
│       ├── main.rs               # Main demonstration
│       ├── why_ub.rs              # Memory layout analysis
│       └── naive_future.rs       # Commented-out broken Future
└── correct-example/
    ├── Cargo.toml
    ├── README.md
    └── src/
        ├── main.rs                # Main entry point
        ├── pin_basics.rs          # Pin fundamentals
        ├── alternative_designs.rs # Non-Pin solutions
        └── async_buf_reader.rs    # Production example
```

## Key Concepts Demonstrated

### The Problem
Self-referential structures (structs with internal pointers) are dangerous because:
1. Rust's move semantics copy bytes to new locations
2. Internal pointers point to old locations
3. Results in dangling pointers and undefined behavior

### The Solutions

**Option 1: Avoid Self-Reference (Preferred)**
- Use indices instead of pointers
- Compute references on demand
- Separate ownership

**Option 2: Use Pin (When Necessary)**
- Mark struct as `!Unpin` with `PhantomPinned`
- Use `Pin<Box<Self>>` to prevent moving
- Use `pin_project` for safe field access

## Example Code Highlights

### Broken (UB)
```rust
struct SelfRef {
    data: String,
    ptr: *const u8,  // DANGER: Points to data's buffer
}
// Moving this invalidates ptr!
```

### Correct (Safe)
```rust
use pin_project::pin_project;

#[pin_project]
struct SelfRef {
    data: String,
    ptr: *const u8,
    _pin: PhantomPinned,  // Prevents moving
}

impl SelfRef {
    fn new(text: &str) -> Pin<Box<Self>> {
        // ... initialization ...
        Box::pin(s)  // Pin immediately
    }
}
```

## Test Coverage

**Alternative Designs:**
- test_index_based_is_movable
- test_separated_ownership

**Pin Basics:**
- test_pin_prevents_move
- test_unpin_allows_get_mut

**Async Buffer Reader:**
- test_basic_read
- test_filled_buffer
- test_consume
- test_pin_prevents_move
- test_multiple_reads

**Total: 9 tests, all passing**

## Environment

- Rust version: 1.85.0 (stable)
- Toolchain: aarch64-apple-darwin
- OS: macOS 15.1.1
- Architecture: Apple Silicon (ARM64)

Tested and verified on:
- macOS 15.1.1 (ARM64)
- Should work on Linux x86_64 and Windows

## Dependencies

**Broken Example:**
- None (demonstrates problems without external deps)

**Correct Example:**
- pin-project 1.1 (safe field projection)
- tokio 1.48 (async runtime)
- futures 0.3 (dev-dependency for tests)

## Submission Details

**Problem Category:** Ownership/Borrowing, Lifetimes, Async/Concurrency, Unsafe, Design Patterns

**Difficulty:** 5/5 (Highest)

**Expected Value:** 200 RMB tier

**Code Quality:**
- All English code and comments
- Comprehensive tests
- Miri-verified (no UB in correct example)
- Production-ready async implementation

## Resources

- [Rust Async Book - Pinning](https://rust-lang.github.io/async-book/04_pinning/01_chapter.html)
- [std::pin documentation](https://doc.rust-lang.org/std/pin/)
- [pin-project crate](https://docs.rs/pin-project/)
- [Pin RFC 2349](https://github.com/rust-lang/rfcs/blob/master/text/2349-pin.md)

## License

MIT License

## Contact

For questions about this example or the Corust.ai submission, see SUBMISSION.md for contact details.
