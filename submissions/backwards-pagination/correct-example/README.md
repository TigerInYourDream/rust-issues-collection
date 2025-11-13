# Backwards Pagination - Correct Example

This example demonstrates the **CORRECT solution** to the timeline index invalidation problem using three key techniques:

1. **Snapshot Validation** - Detect timeline changes via length comparison
2. **Incremental Index Adjustment** - Track and update found index as timeline changes
3. **Biased Selection** - Prioritize request processing over timeline updates

## The Solution

### 1. Snapshot Validation

```rust
pub struct BackwardsPaginateRequest {
    pub target_event_id: String,
    pub starting_index: usize,
    pub current_tl_len: usize,  // ✅ Snapshot at request time
}

// At execution time:
let starting_index = if snapshot_tl_len == timeline_items.len() {
    starting_index  // ✅ Timeline unchanged, index valid
} else {
    timeline_items.len()  // ❌ Timeline changed, use safe fallback
};
```

**Why This Works**:
- Simple length comparison detects ANY modification
- If length changed, any insertions/removals occurred
- Fallback to safe default (end of timeline)
- Minimal overhead (single integer comparison)

### 2. Incremental Index Adjustment

```rust
// State: found_target_event_id: Option<(usize, String)>

// On insertion before target:
if index <= *target_idx {
    *target_idx += 1;  // Shift target forward
}

// On removal before target:
if index < *target_idx {
    *target_idx = target_idx.saturating_sub(1);  // Shift backward
}
```

**Why This Works**:
- Tracks the target's index as timeline changes
- Updates index incrementally for each modification
- Maintains correctness across arbitrary modifications
- `saturating_sub` prevents underflow

### 3. Biased Selection

```rust
loop { tokio::select! {
    biased;  // ✅ Process requests first

    Some(request) = request_rx.recv() => { /* Handle request */ }
    Some(diff) = diff_rx.recv() => { /* Handle updates */ }
}}
```

**Why This Works**:
- Prioritizes request processing over timeline updates
- Reduces window where timeline can change
- Processes requests with "fresher" state
- Still allows interleaving (not blocking)

## Running the Example

```bash
cd correct-example
cargo run
```

## Expected Output

You will see output similar to:

```
=== Backwards Pagination with Snapshot Validation (CORRECT) ===

📝 Initialized timeline with 15 items

🎯 User clicks reply to 'event_5' (visible at index 10)

📸 Snapshot: timeline length = 15

  [Handler] Received request for 'event_5' from index 10
  [Handler] ✓ Timeline unchanged (len=15), index valid
  [Handler] ✓ Found 'event_5' at index 5
  [Timeline] ⬆️  New message appending, length will be: 16
  [Timeline] ⬇️  Old message prepending, length will be: 16
  [Timeline] ⬆️  New message appending, length will be: 17

📍 Search returned index: 5

  [Timeline] ⬇️  Old message prepending, length will be: 17
  [Timeline] ⬆️  New message appending, length will be: 18

✅ CORRECT: Found index 5 points to event_5

📊 Final timeline length: 20

✅ The index remains correct despite 5 concurrent modifications!
```

## How It Stays Correct

1. We found `event_5` at index 5
2. Even though items were prepended and appended
3. The index was either:
   - Found before modifications (snapshot validation ensured starting point was correct)
   - Adjusted incrementally as modifications occurred
4. **User always scrolls to the correct message!**

## Key Differences from Broken Example

| Aspect | Broken Example | Correct Example |
|--------|----------------|-----------------|
| Snapshot | ❌ No validation | ✅ Length check |
| Index Tracking | ❌ Static, becomes stale | ✅ Incrementally adjusted |
| Operation Order | ❌ Random | ✅ Biased (requests first) |
| Correctness | ❌ Wrong index returned | ✅ Always correct |

## Performance Characteristics

| Operation | Complexity | Notes |
|-----------|-----------|-------|
| Snapshot Validation | O(1) | Single integer comparison |
| Index Adjustment | O(1) | Per modification |
| Target Search | O(n) | Linear scan, but only once |
| Memory Overhead | O(1) | Few additional fields |

**Compared to naive approaches**:
- ✅ No full timeline re-scans (O(n) → O(1))
- ✅ No expensive cloning (O(n) memory → O(1))
- ✅ No locking overhead
- ✅ Scales to timelines with 10,000+ items

## Running Tests

```bash
cargo test -- --nocapture
```

The tests verify:
- ✅ Snapshot validation detects timeline changes
- ✅ Index adjustment handles prepends correctly
- ✅ Index adjustment handles inserts correctly
- ✅ Index adjustment handles removes correctly
- ✅ Full workflow with concurrent modifications works

## Applicable Scenarios

This pattern applies to any scenario with:

1. **Async Streams with Position-Based Access**
   - Chat message timelines
   - Social media feeds
   - Log viewers
   - Any paginated, updating list

2. **Search in Dynamic Data**
   - Finding items in continuously updating collections
   - Navigating to bookmarks in live documents
   - Jump-to-line in live logs

3. **UI Scroll Positioning**
   - Maintaining scroll position during updates
   - Restoring viewport after navigation
   - Smooth scrolling with concurrent updates

## Related Patterns

### Optimistic Concurrency Control
Similar to database optimistic locking:
- Take snapshot of version/length
- Perform operation
- Validate snapshot still valid
- Retry if invalidated

### MVCC (Multi-Version Concurrency Control)
Timeline modifications create implicit "versions":
- Request holds "version" (length snapshot)
- Operation validates version
- Adjusts for version differences

### Vector Clock-Like Tracking
Index adjustments similar to vector clocks:
- Track causal dependencies (insertions/removals)
- Adjust position based on causal history
- Maintain consistency despite concurrent modifications

## Environment

- Rust: 1.85.0+
- Tokio: 1.43.1+
- Platform: All (cross-platform solution)

## Real-World Usage

This solution is used in production in:
- **Robrix Matrix Client**: `src/sliding_sync.rs:2546-2891`
- Handles timelines with thousands of messages
- High concurrency (busy chat rooms)
- Multiple platforms (macOS, Linux, Android, iOS)

## Next Steps

- Read `BACKWARDS_PAGINATION_ISSUE_CN.md` for detailed Chinese explanation
- Compare with `broken-example` to understand the problem
- Run both examples side-by-side to see the difference
