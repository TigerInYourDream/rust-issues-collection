# Pin and Self-Referential - Correct Example

This example demonstrates the **correct** way to implement self-referential structures in Rust using Pin.

## Solution Overview

Three approaches are shown:

1. **Alternative Designs** (Preferred) - Avoid self-reference entirely
2. **Pin + pin_project** (When necessary) - Safe self-referential structures
3. **Pin Basics** (Educational) - Understanding Pin concepts

## Key Components

### 1. Pin Basics (`pin_basics.rs`)

Demonstrates fundamental Pin concepts:
- Pin<Box<T>> for heap-pinned values
- PhantomPinned to mark types as !Unpin
- Safe self-referential structs with Pin
- Unpin vs !Unpin

### 2. Alternative Designs (`alternative_designs.rs`)

Shows how to avoid self-reference:
- **Index-based**: Use indices instead of pointers
- **Separated ownership**: Compute references on demand
- **Lazy computation**: Use closures or methods

These are often simpler and safer than using Pin.

### 3. Async Buffer Reader (`async_buf_reader.rs`)

Production-ready example using pin_project:
- Complete AsyncRead implementation
- Safe internal pointer management
- Proper use of pin_project macro
- Full test coverage

## How Pin Solves the Problem

### The Problem
```rust
struct SelfRef {
    data: String,
    ptr: *const u8,  // Points to data's buffer
}
// Moving this struct invalidates ptr!
```

### The Solution
```rust
use pin_project::pin_project;
use std::marker::PhantomPinned;

#[pin_project]
struct SelfRef {
    data: String,
    ptr: *const u8,
    _pin: PhantomPinned,  // Marks as !Unpin
}

// Must use Pin<Box<Self>> to prevent moving
impl SelfRef {
    fn new(text: &str) -> Pin<Box<Self>> {
        // ... initialization ...
        Box::pin(s)  // Pin immediately
    }
}
```

## Key Concepts

### 1. Pin<P>
A type-level guarantee that the value behind pointer P will not move.

### 2. Unpin
A marker trait indicating a type can be safely moved even when pinned.
- Most types are Unpin (automatically)
- Use PhantomPinned to make a type !Unpin

### 3. pin_project
A macro for safe field projection from Pin<&mut Self>:
```rust
#[pin_project]
struct MyStruct {
    #[pin]
    pinned_field: SomeType,    // Pin<&mut SomeType>
    normal_field: OtherType,    // &mut OtherType
}
```

## Running the Examples

```bash
# Run all examples
cargo run

# Run tests
cargo test

# Verify with Miri (UB detector)
cargo +nightly miri test

# Run specific tests
cargo test test_basic_read
cargo test test_pin_prevents_move
```

## When to Use Pin

**Use Pin when:**
- Implementing Future manually
- Creating zero-copy async I/O abstractions
- Building self-referential state machines

**Avoid Pin when:**
- You can redesign without self-reference (preferred!)
- Simple synchronous code
- Can use indices instead of pointers

## Safety Guarantees

With Pin, we guarantee:
1. !Unpin types cannot be moved once pinned
2. Internal pointers remain valid for the lifetime of Pin
3. No undefined behavior from dangling pointers

## Performance

Pin is a **zero-cost abstraction**:
- No runtime overhead
- No additional memory allocation
- Purely compile-time safety

## Best Practices

1. **Prefer alternatives**: Avoid self-reference if possible
2. **Use pin_project**: Don't write unsafe field projection manually
3. **Pin immediately**: Create as Pin<Box<T>> from the start
4. **Document safety**: Explain why unsafe code is safe
5. **Test with Miri**: Verify no undefined behavior

## Environment

- Rust version: 1.85.0 (stable)
- Toolchain: aarch64-apple-darwin (or x86_64-unknown-linux-gnu)
- OS: macOS 15.1.1 / Ubuntu 22.04
- Dependencies:
  - pin-project 1.1
  - tokio 1.48 (with io-util, rt, macros features)

## Related

See `broken-example/` for what NOT to do and why Pin is necessary.

## Resources

- [Rust Async Book - Pinning](https://rust-lang.github.io/async-book/04_pinning/01_chapter.html)
- [std::pin module](https://doc.rust-lang.org/std/pin/)
- [pin-project documentation](https://docs.rs/pin-project/)
- [Pin RFC](https://github.com/rust-lang/rfcs/blob/master/text/2349-pin.md)

## License

MIT
