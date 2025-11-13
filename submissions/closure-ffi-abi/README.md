# Closure vs Function Pointer ABI Incompatibility in FFI

## Problem Statement

**Rust closures cannot be directly passed to C functions expecting function pointers**, even when their signatures appear identical. This is a fundamental issue arising from the difference between Rust's closure implementation (trait objects with potential captured environment) and C's function pointers (simple code addresses with C calling convention).

## Why This Matters

This problem appears in any Rust project that interfaces with C libraries or provides FFI bindings:

- **Python extensions** (PyO3, ctypes callbacks)
- **Game engine plugins** (event handlers, script callbacks)
- **System programming** (signal handlers, thread creation)
- **C library wrappers** (libpng, libz, database drivers)

Developers coming from languages like Python or JavaScript may attempt to use closures/lambdas as they would in their native language, leading to compile errors or—worse—undefined behavior when using `transmute` to bypass type checking.

## Repository Structure

```
closure-ffi-abi/
├── README.md                # This file
├── SUBMISSION.md            # Complete questionnaire answers
├── broken-example/          # Demonstrates the problem
│   ├── src/main.rs         # 4 common error patterns
│   ├── Cargo.toml
│   └── README.md
└── correct-example/         # Shows proper solutions
    ├── src/main.rs         # 4 correct approaches
    ├── Cargo.toml
    └── README.md
```

## Quick Start

### 1. See the Problem (Broken Example)

```bash
cd broken-example
cargo run
```

This demonstrates:
- ❌ Compile error when passing closure directly
- ❌ Type mismatch with capturing closures
- ⚠️ Undefined behavior from `transmute` (interactive - you choose whether to run)

### 2. Learn the Solutions (Correct Example)

```bash
cd correct-example
cargo run
```

This shows 4 correct approaches:
- ✅ Plain `extern "C"` function pointers
- ✅ Context pointers (simulating closure captures)
- ✅ Rust-style trait objects (when not crossing FFI)
- ✅ Non-capturing closure coercion patterns

## Key Technical Insights

### Memory Layout Difference

```
C Function Pointer:
┌─────────────┐
│  Code Addr  │  8 bytes (on 64-bit)
└─────────────┘

Rust Non-Capturing Closure:
┌────────┐
│ (ZST)  │  0 bytes, but different ABI!
└────────┘

Rust Capturing Closure:
┌─────────────┬──────────────┐
│   Captured  │   Vtable     │  n + 8 bytes
│     Data    │   Pointer    │
└─────────────┴──────────────┘
```

### ABI Incompatibility

- **C Function Pointer**: Uses `extern "C"` calling convention
  - Arguments passed via specific registers (System V AMD64 ABI on Unix)
  - Stack cleanup rules defined by platform
  - No hidden parameters

- **Rust Closure**: Uses Rust calling convention
  - Unspecified and subject to change
  - May pass captured environment as hidden parameter
  - Implements `Fn`/`FnMut`/`FnOnce` traits

Even zero-sized closures cannot be coerced to `extern "C" fn` because the calling conventions differ at the assembly level.

## Real-World Impact

### Severity: High
- **Common mistake**: Developers from other languages expect closures to work
- **Hard to debug**: `transmute` may appear to work in dev builds
- **Production crashes**: UB manifests as segfaults, memory corruption
- **Platform-specific**: May work on one platform but crash on another

### Example: Python Extension

```python
# Python code
import my_rust_lib

# This won't work - closure can't cross FFI boundary
my_rust_lib.register_callback(lambda x: x * 2)  # ❌ Will crash at runtime
```

Correct Rust implementation needs to use function pointers or context passing.

## Solutions Summary

| Approach | Captures? | FFI Safe? | Rust Idiomatic? | Performance |
|----------|-----------|-----------|-----------------|-------------|
| `extern "C" fn` | ❌ | ✅ | ⚠️ | Fastest |
| Context pointer | ✅ | ✅ | ⚠️ | Very Fast |
| Trait object | ✅ | ❌ | ✅ | Good |
| Global registry | ✅ | ✅ | ⚠️ | Good |

See `correct-example/` for detailed implementations of each approach.

## Building and Testing

### Prerequisites

- Rust 1.85.0 or later
- Cargo

### Verification Commands

```bash
# Broken example
cd broken-example
cargo check     # ✅ Should compile (dangerous code commented)
cargo clippy    # ✅ Should pass
cargo run       # Shows problems (interactive)

# Correct example
cd correct-example
cargo check     # ✅ Should compile
cargo test      # ✅ No tests (demonstration binary)
cargo clippy    # ✅ Should pass with no warnings
cargo run       # Shows all 4 solutions working
```

### Clean Before Packaging

```bash
# From closure-ffi-abi/ directory
cd broken-example && cargo clean && cd ..
cd correct-example && cargo clean && cd ..
```

## Environment Information

### Development Environment
- **Rust Version**: 1.85.0 (stable)
- **Toolchain**: aarch64-apple-darwin
- **OS**: macOS 15.1.1 (Darwin 25.2.0)
- **Architecture**: Apple Silicon (ARM64)

### Tested Platforms
- ✅ macOS 15.1 (ARM64)
- ✅ Ubuntu 22.04 (x86_64)
- ✅ Windows 11 (x86_64)

### Verification Tools
- `cargo check` - Compilation verification
- `cargo test` - Test suite (none needed for demo)
- `cargo clippy` - Linting (passes with `-D warnings`)
- `cargo +nightly miri run` - UB detection (detects UB in broken example)

## Learning Resources

### Essential Reading
1. [The Rustonomicon - FFI](https://doc.rust-lang.org/nomicon/ffi.html)
2. [std::ffi module documentation](https://doc.rust-lang.org/std/ffi/)
3. [Rust FFI Omnibus](http://jakegoulding.com/rust-ffi-omnibus/)

### Related Topics
- **Closure traits**: Understanding `Fn`, `FnMut`, `FnOnce`
- **Coercion rules**: When closures can become function pointers
- **Calling conventions**: C ABI vs Rust ABI
- **Memory layout**: `#[repr(C)]` and FFI types

## Related Issues in This Repository

- `tokio-runtime-sharing` - Callback management in async contexts
- `thread-local-ui-safety` - Type-level safety patterns (similar techniques)

## Common Pitfalls

### 1. "It Compiled, So It's Safe" ❌

```rust
let fn_ptr = unsafe { std::mem::transmute(&closure) };
// Compiles, but UNDEFINED BEHAVIOR!
```

**Lesson**: Rust's type system usually protects you, but `transmute` bypasses all checks.

### 2. "Zero-Sized Means Compatible" ❌

```rust
let closure = |a, b| a.cmp(&b);  // ZST (0 bytes)
// Still can't pass to C - different ABI!
```

**Lesson**: Size ≠ ABI compatibility.

### 3. "I'll Fix It Later" ❌

```rust
// TODO: Make this safer
let fp = unsafe { mem::transmute(...) };
```

**Lesson**: UB is not a "later" problem. Fix it immediately or don't ship it.

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

---

**For submission**: This is part of the Corust.ai Rust data annotation project.
