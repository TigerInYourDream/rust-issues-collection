# Closure FFI ABI Incompatibility - Broken Example

## Problem Overview

This example demonstrates why Rust closures cannot be directly passed to C functions expecting function pointers, even when their signatures appear compatible.

## What's Wrong

Rust closures and C function pointers have fundamentally different:

1. **Memory Layout**
   - Function pointer: Just a code address (8 bytes on 64-bit)
   - Closure: May include captured environment data
   - Even zero-sized closures use different calling conventions

2. **ABI (Application Binary Interface)**
   - C expects: `extern "C"` calling convention
   - Closures use: Rust calling convention (unspecified)
   - The two are incompatible at the assembly level

3. **Type System**
   - Function pointers: Implement `Copy`
   - Closures: Implement `Fn`/`FnMut`/`FnOnce` traits
   - Different trait implementations = different types

## Demonstrated Problems

This example shows 4 common mistakes:

### Problem 1: Direct Closure Passing (Compile Error)

```rust
qsort(..., |a, b| a.cmp(&b));  // ❌ Type mismatch
```

**Result**: Compile error - type system protects us.

### Problem 2: Capturing Closure (Memory Layout Issue)

```rust
let reverse = true;
let comparator = |a, b| if reverse { ... };  // Captures 'reverse'
```

**Result**: Cannot even attempt to pass - has non-trivial memory layout.

### Problem 3: Dangerous Transmute (Undefined Behavior)

```rust
let fn_ptr = std::mem::transmute(&closure);  // ⚠️ Compiles but UB!
qsort(..., fn_ptr);  // 💥 May crash or corrupt memory
```

**Result**:
- Sometimes crashes immediately (segfault)
- Sometimes appears to work (worst case - silent corruption)
- Always undefined behavior according to Rust spec

### Problem 4: ZST Confusion (ABI Mismatch)

Even zero-sized (non-capturing) closures can't be used because:
- Rust closure ABI ≠ C function pointer ABI
- Different calling conventions at assembly level
- Size alone doesn't determine compatibility

## How to Run

### Safe Mode (Recommended)

```bash
cargo run
```

This will:
1. Show explanations of each problem
2. Print memory layout differences
3. Pause before running dangerous code

### Expected Output

```
╔════════════════════════════════════════════════════════════════════╗
║          Closure vs Function Pointer FFI Problem                  ║
║                    (Broken Examples)                               ║
╚════════════════════════════════════════════════════════════════════╝

ERROR: Closures cannot be coerced to function pointers!
ERROR: Capturing closures have non-trivial memory layout

=== Understanding Closure Layout ===
Non-capturing closure size: 0 bytes
Capturing closure size: 4 bytes
Function pointer size: 8 bytes

Key insight: Even zero-sized closures have different ABI!

╔════════════════════════════════════════════════════════════════════╗
║  Why Closures Don't Work as C Function Pointers                   ║
╚════════════════════════════════════════════════════════════════════╝

⚠️  Press Enter to run DANGEROUS transmute attempt (may crash)...
```

### Dangerous Code Warning

The example includes code that uses `transmute` to force a closure into a function pointer. This:
- **May crash** your process (segmentation fault)
- **May corrupt memory** silently
- **Is undefined behavior** according to Rust specification
- Is included only to demonstrate why this approach is dangerous

## Building and Testing

```bash
# Check compilation
cargo check

# Run with explanations
cargo run

# Check with Miri (UB detection)
cargo +nightly miri run  # Will detect UB in transmute example
```

## Environment

- **Rust Version**: 1.85.0
- **Toolchain**: aarch64-apple-darwin
- **OS**: macOS 15.1.1
- **Architecture**: ARM64 (Apple Silicon)

Also tested on:
- x86_64-unknown-linux-gnu (Ubuntu 22.04)
- x86_64-pc-windows-msvc (Windows 11)

## Key Takeaways

1. ❌ **Never** use `transmute` to convert closures to function pointers
2. ❌ Closures and function pointers are fundamentally different types
3. ✅ The type system prevents most mistakes at compile time
4. ✅ See `../correct-example/` for proper solutions

## Related Documentation

- [The Rustonomicon - FFI](https://doc.rust-lang.org/nomicon/ffi.html)
- [std::mem::transmute](https://doc.rust-lang.org/std/mem/fn.transmute.html) - Read the safety section!
- [Rust Reference - Closures](https://doc.rust-lang.org/reference/types/closure.html)

## License

MIT
