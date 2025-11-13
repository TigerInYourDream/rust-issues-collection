# Rust Issues Collection

A comprehensive collection of real-world Rust common issues and their solutions, created for the [Corust.ai](https://corust.ai) data annotation project.

## 🎯 Project Overview

This repository contains carefully curated examples of common pitfalls and best practices in Rust programming. Each issue includes:

- **Broken Example**: Demonstrates the problem with clear explanations
- **Correct Example**: Shows multiple solutions with detailed documentation
- **Complete Documentation**: English README files with environment information

## 📚 Issue Categories

### FFI & Foreign Function Interface
- [**Closure FFI ABI Incompatibility**](submissions/closure-ffi-abi/) - Why Rust closures can't be passed to C function pointers

### Async & Concurrency
- [**Tokio Runtime Sharing**](submissions/tokio-runtime-sharing/) - Safe runtime sharing with CancellationToken
- [**Async Drop Trap**](submissions/async-drop-trap/) - Graceful shutdown patterns for async resources
- [**Tokio Signal FD Leak**](submissions/tokio-signal-fd-leak/) - Signal handler file descriptor management
- [**Tokio Select Spin**](submissions/tokio-select-spin/) - Avoiding CPU spin in select! macros

### Memory Safety & Ownership
- [**Pin & Self-Referential Structures**](submissions/pin-self-referential/) - Safe self-referential types with Pin
- [**RefCell Double Borrow Panic**](submissions/refcell-double-borrow-panic/) - Interior mutability patterns
- [**Thread-Local UI Safety**](submissions/thread-local-ui-safety/) - Witness type pattern for thread safety

### Type Safety & Design Patterns
- [**Room DisplayName Type Safety**](submissions/room-displayname-type-safety/) - NewType pattern for type safety

### Performance & Platform-Specific
- [**AVX2 Feature Detection**](submissions/target-feature-illegal-instruction/) - Runtime CPU feature detection
- [**Backwards Pagination**](submissions/backwards-pagination/) - Efficient reverse iteration patterns

## 🚀 Quick Start

Each issue is structured as a standalone Cargo project:

```bash
# Clone the repository
git clone https://github.com/TigerInYourDream/rust-issues-collection.git
cd rust-issues-collection

# Explore a specific issue
cd submissions/closure-ffi-abi

# Run the broken example (demonstrates the problem)
cd broken-example
cargo run

# Run the correct example (shows solutions)
cd ../correct-example
cargo run
```

## ✅ Quality Standards

All submissions pass:
- `cargo check` - Compilation verification
- `cargo test` - Test suite (where applicable)
- `cargo clippy` - Linting with no warnings

Code quality:
- ✅ All code and comments in English
- ✅ Comprehensive documentation
- ✅ Environment information included
- ✅ MIT or other open-source licenses

## 🏗️ Project Structure

```
rust-issues-collection/
├── submissions/           # Collection of issue examples
│   ├── closure-ffi-abi/
│   │   ├── README.md     # Issue overview
│   │   ├── broken-example/
│   │   │   ├── src/
│   │   │   ├── Cargo.toml
│   │   │   └── README.md
│   │   └── correct-example/
│   │       ├── src/
│   │       ├── Cargo.toml
│   │       └── README.md
│   └── ... (other issues)
└── rust-logout-issue/     # Main example
```

## 🎓 Learning Resources

Each issue includes:
- **Problem Background**: Real-world scenario where the issue occurs
- **Code Examples**: Minimal reproducible examples
- **Error Messages**: Complete compiler/runtime errors
- **Solutions**: Multiple approaches with trade-offs
- **Best Practices**: Recommended patterns and anti-patterns

## 🤝 Contributing

This repository is part of the Corust.ai data annotation project. While direct contributions may be limited, you can:

1. **Star** ⭐ this repository if you find it helpful
2. **Share** with others learning Rust
3. **Open issues** for questions or clarifications
4. **Fork** and adapt for your own learning

## 📊 Issue Statistics

- **Total Issues**: 11+
- **Categories**: FFI, Async, Memory Safety, Type Safety, Performance
- **Lines of Code**: 14,000+
- **Documentation**: Comprehensive English READMEs

## 🏆 Highlight: Closure FFI ABI Incompatibility

One of the most valuable submissions demonstrates why **Rust closures cannot be passed directly to C function pointers**:

```rust
// ❌ WRONG: This won't compile
qsort(array, len, size, |a, b| a.cmp(&b));

// ✅ CORRECT: Use extern "C" fn
extern "C" fn compare(a: *const c_void, b: *const c_void) -> c_int {
    // Implementation
}
qsort(array, len, size, compare);
```

**Why?** Rust closures and C function pointers have different:
- Memory layouts
- Calling conventions (Rust ABI vs C ABI)
- Type system representations

[See full example →](submissions/closure-ffi-abi/)

## 🌟 Featured Issues

### For Beginners
- RefCell Double Borrow Panic
- Thread-Local UI Safety
- Room DisplayName Type Safety

### For Intermediate Developers
- Closure FFI ABI Incompatibility
- Async Drop Trap
- Tokio Runtime Sharing

### For Advanced Developers
- Pin & Self-Referential Structures
- AVX2 Feature Detection
- Tokio Signal FD Leak

## 📝 License

Each submission may have its own license (typically MIT). See individual project directories for details.

## 🔗 Related Projects

- [Corust.ai](https://corust.ai) - AI Coding Agent for Rust
- [The Rust Programming Language Book](https://doc.rust-lang.org/book/)
- [Rustonomicon](https://doc.rust-lang.org/nomicon/) - The Dark Arts of Unsafe Rust
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)

## 📮 Contact

For questions about this repository or the Corust.ai project:
- GitHub Issues: [Create an issue](https://github.com/TigerInYourDream/rust-issues-collection/issues)
- Corust.ai: daogangtang at qq.com

---

**Made with ❤️ for the Rust community**

*This repository helps developers avoid common pitfalls and learn Rust best practices through real-world examples.*
