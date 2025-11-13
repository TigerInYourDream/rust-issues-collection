# RefCell Double Borrow Panic - Broken Example

This project demonstrates two common scenarios where `RefCell<T>` panics at runtime due to borrow rule violations.

## Problem Summary

`RefCell<T>` provides interior mutability with **runtime** borrow checking. Unlike compile-time checks, RefCell panics at runtime if you violate borrowing rules:
- Cannot borrow mutably while an immutable borrow exists
- Cannot borrow while a mutable borrow exists

**The danger**: Code compiles fine but crashes in production when specific execution paths are triggered.

## Two Common Scenarios

### Scenario 1: Modifying Collection While Iterating

When processing items in a collection and the callback tries to add new items:

```rust
// Holds immutable borrow
for item in cache.borrow().iter() {
    if item.contains("special") {
        add_to_cache(new_item);  // Tries mutable borrow - PANIC!
    }
}
```

**Why it panics**: You can't read and write at the same time.

### Scenario 2: Function Call Chain on Same RefCell

When one function calls another function that borrows the same RefCell:

```rust
COUNTER.with(|c| {
    let mut counter = c.borrow_mut();  // Mutable borrow
    *counter += 1;
    let value = get_value();  // Tries to borrow again - PANIC!
});
```

**Why it panics**: The first borrow is still active when you try to borrow again.

## Running the Example

```bash
cargo run
```

The program runs successfully because panic-causing code is commented out.

To see actual panics, uncomment the problematic code blocks (lines marked with PANIC comments).

## Running Tests

```bash
cargo test
```

Tests include `#[should_panic]` tests that demonstrate the actual runtime panics.

## Key Takeaways

- **Compiles but panics** - No compile-time warning
- **Hard to detect** - Only triggers on specific code paths
- **Common in single-threaded apps** - GUI, games, event systems
- **Production risk** - Can appear suddenly in production

## Environment

- Rust version: 1.85.0 (stable)
- Toolchain: aarch64-apple-darwin
- OS: macOS 15.1.1

## License

MIT
