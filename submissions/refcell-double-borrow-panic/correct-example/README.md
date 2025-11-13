# RefCell Double Borrow Panic - Correct Solutions

This project demonstrates three simple solutions for using `RefCell<T>` safely without runtime panics.

## Problem Recap

RefCell provides interior mutability with runtime borrow checking. The problem: borrowing while already holding a borrow causes runtime panic.

## Three Simple Solutions

### Solution 1: Clone and Release (Most Common)

**When to use**: Processing collections where callbacks might modify the collection

**How it works**: Clone the data, release the borrow, then process safely

```rust
// Clone data, borrow is released immediately
let items = CACHE.with(|cache| cache.borrow().clone());

// Now safe to modify CACHE
for item in items {
    if item.contains("special") {
        add_to_cache(format!("derived-{}", item));  // Safe!
    }
}
```

**Pros**: Simple and always safe
**Cons**: Memory overhead from cloning

### Solution 2: Single Borrow Scope

**When to use**: Simple operations that don't need to call other functions

**How it works**: Do everything within one borrow scope

```rust
COUNTER.with(|c| {
    let mut counter = c.borrow_mut();
    *counter += 1;
    // Use the value directly, don't call other functions
    println!("Value: {}", *counter);
}); // Borrow released here
```

**Pros**: Zero overhead
**Cons**: Can't split logic across functions

### Solution 3: Use Cell for Copy Types

**When to use**: Simple scalar values (integers, booleans, floats)

**How it works**: Cell doesn't use borrowing at all

```rust
struct Counter {
    value: Cell<i32>,  // Use Cell instead of RefCell
}

impl Counter {
    fn increment(&self) {
        self.value.set(self.value.get() + 1);  // No borrowing!
    }

    fn update_and_log(&self) {
        self.increment();
        let value = self.get();  // No panic possible!
        println!("Counter: {}", value);
    }
}
```

**Pros**: Zero overhead, impossible to panic
**Cons**: Only works with Copy types (i32, bool, etc)

## Quick Comparison

| Solution | Use Case | Memory Cost | Code Complexity |
|----------|----------|-------------|-----------------|
| Clone & Release | Collections, complex processing | High | Low |
| Single Borrow | Simple operations | None | Medium |
| Cell | Counters, flags, scalars | None | Low |

## Running the Example

```bash
cargo run
```

Shows all three solutions working correctly.

## Running Tests

```bash
cargo test
```

All 4 tests pass, demonstrating safe RefCell usage.

## Key Takeaways

1. **Clone when in doubt** - Simplest and safest approach
2. **Keep borrows short** - Release as soon as possible
3. **Use Cell for scalars** - Perfect for counters and flags
4. **Avoid nested borrows** - Don't call functions that might borrow again

## Environment

- Rust version: 1.85.0 (stable)
- Toolchain: aarch64-apple-darwin
- OS: macOS 15.1.1

## References

- [Rust Book - Interior Mutability](https://doc.rust-lang.org/book/ch15-05-interior-mutability.html)
- [RefCell Documentation](https://doc.rust-lang.org/std/cell/struct.RefCell.html)
- [Cell Documentation](https://doc.rust-lang.org/std/cell/struct.Cell.html)

## License

MIT
