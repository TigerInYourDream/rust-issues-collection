# Pin and Self-Referential - Broken Example

This example demonstrates the problems with self-referential structures in Rust.

## Problem Overview

Self-referential structures (where a field points to another field in the same struct) are dangerous in Rust because:

1. **Rust's move semantics**: When a struct is moved, its bytes are copied to a new location
2. **Internal pointers become dangling**: Pointers that point to the old location are now invalid
3. **Undefined behavior**: Reading from dangling pointers is UB

## What This Example Shows

### Example 1: Simple Move
- Creates a self-referential struct
- Moves it to a new variable
- Demonstrates that internal pointers can become stale

### Example 2: Vec Reallocation
- Puts self-referential structs in a Vec
- Forces reallocation by adding elements
- Shows how reallocation moves elements, breaking internal pointers

### Example 3: Lifetime Attempts (Doesn't Compile)
- Shows why you cannot use lifetimes to solve this
- Rust's borrow checker correctly prevents this

### Example 4: Detailed Memory Analysis
- Shows memory addresses before and after moves
- Demonstrates when pointers become dangling
- Explains why reassignment breaks everything

## Running the Examples

```bash
# Run the examples (may produce garbage output or crash)
cargo run

# Detect undefined behavior with Miri
cargo +nightly miri run
```

## Expected Behavior

**WARNING**: This code exhibits undefined behavior!

- May print garbage data
- May segfault
- May appear to work (by luck)
- Miri will detect the UB

## Key Takeaways

1. **Raw pointers are not enough**: Even with raw pointers, moves invalidate them
2. **Lifetimes cannot express self-reference**: `&'a` must point to data with lifetime `'a`, not self
3. **Pin is necessary**: To safely have self-referential structures, you need Pin
4. **This is why async works**: `async` generates self-referential state machines, which require Pin

## Environment

- Rust version: 1.85.0 (stable)
- Toolchain: aarch64-apple-darwin (or x86_64-unknown-linux-gnu)
- OS: macOS 15.1.1 / Ubuntu 22.04

## Related

See `correct-example/` for the proper solution using Pin and pin_project.

## License

MIT
