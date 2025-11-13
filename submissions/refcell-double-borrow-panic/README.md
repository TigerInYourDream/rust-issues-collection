# RefCell Double Borrow Panic Trap

**SIMPLIFIED EXAMPLES** - Clear demonstrations of RefCell runtime panics and their solutions.

## Problem Overview

`RefCell<T>` provides interior mutability with **runtime** borrow checking. While Rust's motto is "if it compiles, it's safe," RefCell can still **panic at runtime**:

- Cannot borrow mutably while an immutable borrow exists
- Cannot borrow (mutably or immutably) while a mutable borrow exists

**The danger**: Code compiles fine, but panics in production when users trigger specific code paths.

## Project Structure

```
refcell-double-borrow-panic/
├── README.md                    # This file
├── broken-example/              # 2 common panic scenarios
│   ├── src/main.rs             # ~180 lines, simple examples
│   └── README.md
└── correct-example/             # 3 simple solutions
    ├── src/main.rs             # ~230 lines, clear solutions
    └── README.md
```

## Two Common Scenarios (Broken Example)

### Scenario 1: Modifying Collection While Iterating

```rust
// PROBLEM: Borrow while already borrowed
for item in cache.borrow().iter() {  // Immutable borrow
    if item.contains("special") {
        add_to_cache(new_item);      // Tries to mutably borrow - PANIC!
    }
}
```

**Why it panics**: You're reading (immutable borrow) and trying to write (mutable borrow) at the same time.

### Scenario 2: Function Calls on Same RefCell

```rust
// PROBLEM: Nested function calls
COUNTER.with(|c| {
    let mut counter = c.borrow_mut();  // Mutable borrow
    *counter += 1;

    let value = get_value();  // Tries to borrow again - PANIC!
});
```

**Why it panics**: The first borrow is still active when you try to borrow again.

## Three Simple Solutions (Correct Example)

### Solution 1: Clone and Release (Simplest!)

```rust
// Copy data, release borrow, then process
let items = cache.borrow().clone();  // Borrow released here!

for item in items {
    add_to_cache(new_item);  // Safe! No active borrow
}
```

**Trade-off**: Uses memory to clone data

### Solution 2: Do Everything in One Borrow

```rust
// Don't call other functions, do everything here
COUNTER.with(|c| {
    let mut counter = c.borrow_mut();
    *counter += 1;
    println!("Value: {}", *counter);  // Direct access, no function call
}); // Borrow released here
```

**Trade-off**: Can't split logic into multiple functions

### Solution 3: Use Cell for Simple Types

```rust
struct Counter {
    value: Cell<i32>,  // No borrowing needed!
}

fn update(&self) {
    self.value.set(self.value.get() + 1);  // No panic possible!
    let v = self.value.get();               // No borrowing!
}
```

**Limitation**: Only works with Copy types (i32, bool, etc)

## Running the Examples

### Broken Example
```bash
cd broken-example
cargo run      # Shows where panics would occur (commented out)
cargo test     # Includes tests that actually panic
```

### Correct Example
```bash
cd correct-example
cargo run      # Shows all 3 solutions working
cargo test     # All tests pass
```

## Quick Comparison

| Approach | Memory Cost | Code Simplicity | Limitations |
|----------|-------------|-----------------|-------------|
| Clone & Release | High (clone) | Simple | Needs Clone trait |
| Single Borrow | None | Medium | Must inline logic |
| Cell | None | Simple | Copy types only |

## Key Takeaways

1. **RefCell panics at runtime** - Compiles fine, crashes later
2. **Avoid nested borrows** - Especially across function boundaries
3. **Clone when in doubt** - Simple and safe
4. **Cell for scalars** - Perfect for counters, flags, etc
5. **Keep borrows short** - Release as soon as possible

## When to Use RefCell

**Good uses**:
- Single-threaded GUI state management
- Caching and memoization
- Simple interior mutability patterns

**Avoid when**:
- You can use `&mut self` instead
- Multi-threaded context (use `Mutex`)
- Complex calling patterns
- Copy types (use `Cell`)

## Environment

- **Rust version**: 1.85.0 (stable)
- **Toolchain**: aarch64-apple-darwin
- **OS**: macOS 15.1.1

All examples pass `cargo check`, `cargo test`, and `cargo clippy`.

## References

- [Rust Book - Interior Mutability](https://doc.rust-lang.org/book/ch15-05-interior-mutability.html)
- [std::cell::RefCell](https://doc.rust-lang.org/std/cell/struct.RefCell.html)
- [std::cell::Cell](https://doc.rust-lang.org/std/cell/struct.Cell.html)

## License

MIT
