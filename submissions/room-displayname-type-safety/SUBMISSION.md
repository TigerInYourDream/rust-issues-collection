# Rust Issue Submission Template

## Metadata
- **Category**: [Select from list below]
  - [ ] Ownership & Borrowing
  - [ ] Lifetime Management
  - [ ] Async/Concurrency
  - [ ] Performance Optimization
  - [ ] Error Handling
  - [ ] Type System
  - [ ] FFI
  - [ ] Traits & Generics
  - [ ] Macros
  - [ ] Design Patterns
  - [ ] Other: ___________

- **Difficulty**: ⭐⭐⭐ (1-5 stars)
- **Rarity**: Common / Moderate / Rare
- **Expected Value**: 30 / 50 / 100 / 200 RMB

## Problem Title (English)
> Short, descriptive title (e.g., "Lifetime conflict when sharing room state across async tasks")

## Problem Background (English)
> Describe the real-world scenario where this problem occurred.
> - What were you trying to build?
> - What was the expected behavior?
> - What went wrong?

**Example:**
```
When building a Matrix client, I needed to share room information
between multiple async tasks that handle UI updates and network sync.
The compiler rejected my initial approach due to lifetime conflicts.
```

## Incorrect Code Example (English comments)
> Provide the broken code that demonstrates the problem

```rust
// TODO: Add your broken example here
```

**Compiler Error:**
```
// Paste the exact compiler error message
```

## Root Cause Analysis (English)
> Explain WHY this error occurs
> - What Rust concept/rule is violated?
> - Why does the compiler reject this?

## Solution (English)
> Explain the correct approach and WHY it works

## Correct Code Example (English comments)
> Full working Cargo project structure

```rust
// TODO: Add your working example here
```

## Key Takeaways (English)
> List 2-3 important lessons learned

1.
2.
3.

## Related Patterns/Best Practices
> Optional: mention related design patterns or idioms

---

## Checklist Before Submission
- [ ] Code is 100% English (comments, variable names, etc.)
- [ ] No proprietary/private code included
- [ ] Checked deduplication sheet
- [ ] Created full Cargo project for correct example
- [ ] Tested that broken code actually fails
- [ ] Tested that correct code compiles and runs
- [ ] Added clear comments explaining the issue
