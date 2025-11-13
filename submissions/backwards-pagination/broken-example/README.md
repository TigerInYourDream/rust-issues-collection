# Backwards Pagination - Broken Example

This example demonstrates the **index invalidation problem** that occurs when searching for events in a timeline that is being concurrently modified.

## The Problem

When implementing "jump to message" functionality (e.g., clicking on a reply preview):

1. User clicks to view a message at index 50
2. System starts searching backwards from index 50
3. **Meanwhile**: New messages arrive (appended to timeline end)
4. **Meanwhile**: Pagination loads old messages (prepended to timeline start)
5. **Result**: The index we found is NO LONGER VALID!

## What This Example Shows

### Three Critical Flaws

1. **Index Staleness**
   - The `starting_index` parameter may already be outdated when we start processing
   - No validation that the timeline hasn't changed since the request

2. **No Modification Detection**
   - We cannot detect if the timeline was modified during our search
   - No snapshot mechanism to track timeline state

3. **Race Windows**
   - Between releasing read lock and acquiring it again, arbitrary modifications occur
   - The found index becomes stale immediately

## Running the Example

```bash
cd broken-example
cargo run
```

## Expected Output

You will see output similar to:

```
=== Backwards Pagination Index Invalidation Demo (BROKEN) ===

📝 Initialized timeline with 15 items

🎯 User clicks reply to 'event_5' (visible at index 10)

  [Search] Searching from index 10...
  [Timeline] ⬆️  New message appended, length now: 16
  [Search] ✓ Found target at index 5
  [Timeline] ⬇️  Old message prepended, length now: 17

📍 Search returned index: 5

  [Timeline] ⬆️  New message appended, length now: 18
  [Timeline] ⬇️  Old message prepended, length now: 19

❌ WRONG: Found index 5 points to old_message_0, expected event_5

📊 Final timeline length: 19

⚠️  The index was valid when found, but became invalid due to concurrent modifications!
```

## The Bug Explained

1. We found `event_5` at index 5 (correct at that moment)
2. But then 1 item was prepended to the timeline
3. Now `event_5` is actually at index 6, not 5!
4. Index 5 now points to the wrong message
5. **User will scroll to the wrong message!**

## Why This Compiles Without Errors

This is a **logic bug**, not a compiler error:
- No unsafe code
- No data races (RwLock protects the data)
- No type errors

But the behavior is **wrong** - users see the wrong messages!

## Running Tests

```bash
cargo test -- --nocapture
```

The tests demonstrate:
- Index invalidation due to prepended items
- Lack of snapshot validation
- How concurrent modifications break correctness

## Environment

- Rust: 1.85.0+
- Tokio: 1.43.1+
- Platform: All (cross-platform logic bug)

## Next Steps

See the `correct-example` directory for the solution using:
1. Snapshot validation (timeline length check)
2. Incremental index adjustment tracking
3. Biased selection for operation ordering
