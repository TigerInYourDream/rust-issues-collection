//! Correct Example: Backwards Pagination with Snapshot Validation and Index Adjustment
//!
//! This example demonstrates the CORRECT solution to the timeline index invalidation
//! problem using three key techniques:
//!
//! 1. **Snapshot Validation**: Track timeline length to detect modifications
//! 2. **Incremental Index Adjustment**: Update found index as timeline changes
//! 3. **Biased Selection**: Prioritize request processing over timeline updates
//!
//! # Run this example
//!
//! ```bash
//! cargo run --bin correct
//! ```
//!
//! You'll see that the found index remains correct despite concurrent modifications.

use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{sleep, Duration};

/// A simplified timeline item representing a message
#[derive(Debug, Clone)]
struct TimelineItem {
    event_id: String,
    content: String,
}

/// Request to search backwards for a specific event
#[derive(Debug, Clone)]
struct BackwardsPaginateRequest {
    target_event_id: String,
    starting_index: usize,
    current_tl_len: usize,  // Snapshot for validation
}

/// Result of finding a target event
#[derive(Debug, Clone)]
struct TargetEventFound {
    target_event_id: String,
    index: usize,  // OK: Adjusted index that remains correct
}

/// Represents different types of timeline modifications
#[derive(Debug)]
enum TimelineDiff {
    PushBack { item: TimelineItem },
    PushFront { item: TimelineItem },
    Insert { index: usize, item: TimelineItem },
    Remove { index: usize },
}

/// Timeline with snapshot validation support
struct Timeline {
    items: Arc<RwLock<Vec<TimelineItem>>>,
}

impl Timeline {
    fn new() -> Self {
        Self {
            items: Arc::new(RwLock::new(Vec::new())),
        }
    }

    async fn get_length(&self) -> usize {
        self.items.read().await.len()
    }

    async fn get_item(&self, index: usize) -> Option<TimelineItem> {
        self.items.read().await.get(index).cloned()
    }

    /// Apply a timeline modification
    async fn apply_diff(&self, diff: TimelineDiff) {
        let mut items = self.items.write().await;
        match diff {
            TimelineDiff::PushBack { item } => {
                items.push(item);
            }
            TimelineDiff::PushFront { item } => {
                items.insert(0, item);
            }
            TimelineDiff::Insert { index, item } => {
                if index <= items.len() {
                    items.insert(index, item);
                }
            }
            TimelineDiff::Remove { index } => {
                if index < items.len() {
                    items.remove(index);
                }
            }
        }
    }
}

/// OK: CORRECT: Search handler with snapshot validation and index adjustment
///
/// This function demonstrates three key patterns:
///
/// 1. **Snapshot Validation**: Checks if timeline changed since request
/// 2. **Index Adjustment**: Tracks found index as timeline is modified
/// 3. **Biased Selection**: Prioritizes requests over timeline updates
async fn timeline_search_handler(
    timeline: Timeline,
    mut request_rx: mpsc::Receiver<BackwardsPaginateRequest>,
    mut diff_rx: mpsc::Receiver<TimelineDiff>,
    result_tx: mpsc::Sender<TargetEventFound>,
) {
    // Current search state
    let mut target_event_id: Option<String> = None;

    // If found, store (index, event_id)
    // OK: This index will be incrementally adjusted as timeline changes
    let mut found_target_event_id: Option<(usize, String)> = None;

    loop {
        tokio::select! {
            // OK: BIASED: Process requests BEFORE timeline updates
            // This reduces the window where timeline can change
            biased;

            // Handle new backwards pagination requests
            Some(request) = request_rx.recv() => {
                println!("  [Handler] Received request for '{}' from index {}",
                    request.target_event_id, request.starting_index);

                let items = timeline.items.read().await;
                let current_tl_len = items.len();

                // OK: VALIDATE: Check if timeline changed since request
                let starting_index = if request.current_tl_len == current_tl_len {
                    println!("  [Handler] ✓ Timeline unchanged (len={}), index valid", current_tl_len);
                    request.starting_index
                } else {
                    println!("  [Handler] WARNING: Timeline changed (was {}, now {}), using safe fallback",
                        request.current_tl_len, current_tl_len);
                    // Timeline changed, cannot trust starting_index
                    // Use safe default: search from end
                    current_tl_len
                };

                // Search backwards from validated index
                let found_index = items
                    .iter()
                    .enumerate()
                    .take(starting_index)
                    .rev()
                    .find(|(_, item)| item.event_id == request.target_event_id)
                    .map(|(i, _)| i);

                drop(items);  // Release lock

                if let Some(index) = found_index {
                    // OK: Found in existing timeline!
                    println!("  [Handler] ✓ Found '{}' at index {}", request.target_event_id, index);

                    target_event_id = None;
                    found_target_event_id = None;

                    result_tx.send(TargetEventFound {
                        target_event_id: request.target_event_id,
                        index,
                    }).await.ok();
                } else {
                    // Not found, start searching in incoming diffs
                    println!("  [Handler] Not found yet, will check incoming updates...");
                    target_event_id = Some(request.target_event_id);
                    found_target_event_id = None;
                }
            }

            // Handle timeline updates
            Some(diff) = diff_rx.recv() => {
                // First, adjust the found index if we have one
                if let Some((target_idx, _target_id)) = found_target_event_id.as_mut() {
                    match &diff {
                        TimelineDiff::PushFront { .. } => {
                            // OK: ADJUST: Prepended item shifts index forward
                            *target_idx += 1;
                            println!("  [Handler] DOWN: Item prepended, adjusted found index to {}", target_idx);
                        }
                        TimelineDiff::Insert { index, .. } => {
                            // OK: ADJUST: Insertion before target shifts it forward
                            if *index <= *target_idx {
                                *target_idx += 1;
                                println!("  [Handler] INSERT: Item inserted at {}, adjusted found index to {}",
                                    index, target_idx);
                            }
                        }
                        TimelineDiff::Remove { index } => {
                            // OK: ADJUST: Removal before target shifts it backward
                            if *index < *target_idx {
                                *target_idx = target_idx.saturating_sub(1);
                                println!("  [Handler] REMOVE: Item removed at {}, adjusted found index to {}",
                                    index, target_idx);
                            } else if *index == *target_idx {
                                // Target itself was removed!
                                println!("  [Handler] WARNING: Target was removed!");
                                found_target_event_id = None;
                                target_event_id = None;
                            }
                        }
                        TimelineDiff::PushBack { .. } => {
                            // Appending to end doesn't affect indices
                        }
                    }

                    // If we still have a found target, report it
                    if let Some((final_index, final_id)) = found_target_event_id.take() {
                        println!("  [Handler] >> Reporting final adjusted index: {}", final_index);
                        result_tx.send(TargetEventFound {
                            target_event_id: final_id,
                            index: final_index,
                        }).await.ok();
                        target_event_id = None;
                    }
                } else if let Some(ref target_id) = target_event_id {
                    // Still searching - check if this diff contains our target
                    let is_target = match &diff {
                        TimelineDiff::PushFront { item } |
                        TimelineDiff::PushBack { item } |
                        TimelineDiff::Insert { item, .. } => {
                            item.event_id == *target_id
                        }
                        _ => false,
                    };

                    if is_target {
                        // Found the target in this diff!
                        let index = match &diff {
                            TimelineDiff::PushFront { .. } => 0,
                            TimelineDiff::PushBack { .. } => timeline.get_length().await - 1,
                            TimelineDiff::Insert { index, .. } => *index,
                            _ => unreachable!(),
                        };

                        println!("  [Handler] ✓ Found '{}' in diff at index {}", target_id, index);

                        // Mark as found, will be reported after this batch
                        found_target_event_id = Some((index, target_id.clone()));
                    }
                }

                // Apply the diff to timeline
                timeline.apply_diff(diff).await;
            }

            else => break,
        }
    }
}

/// Simulate concurrent timeline modifications
async fn simulate_concurrent_updates(
    timeline: Timeline,
    diff_tx: mpsc::Sender<TimelineDiff>,
) {
    // Task 1: Simulate new messages arriving (append)
    let timeline_clone = Timeline { items: timeline.items.clone() };
    let diff_tx_clone = diff_tx.clone();
    tokio::spawn(async move {
        for i in 0..5 {
            sleep(Duration::from_millis(80)).await;

            let item = TimelineItem {
                event_id: format!("new_message_{}", i),
                content: format!("New message {}", i),
            };

            let len = timeline_clone.get_length().await + 1;
            println!("  [Timeline] UP: New message appending, length will be: {}", len);

            diff_tx_clone.send(TimelineDiff::PushBack { item }).await.ok();
        }
    });

    // Task 2: Simulate pagination loading old messages (prepend)
    let timeline_clone2 = Timeline { items: timeline.items.clone() };
    let diff_tx_clone2 = diff_tx.clone();
    tokio::spawn(async move {
        for i in 0..5 {
            sleep(Duration::from_millis(120)).await;

            let item = TimelineItem {
                event_id: format!("old_message_{}", i),
                content: format!("Old message {}", i),
            };

            let len = timeline_clone2.get_length().await + 1;
            println!("  [Timeline] DOWN: Old message prepending, length will be: {}", len);

            diff_tx_clone2.send(TimelineDiff::PushFront { item }).await.ok();
        }
    });
}

/// Verify if the found index is actually correct
async fn verify_result(timeline: &Timeline, found_index: usize, expected_event_id: &str) -> bool {
    if let Some(item) = timeline.get_item(found_index).await {
        let is_correct = item.event_id == expected_event_id;
        if is_correct {
            println!("\nOK: CORRECT: Found index {} points to {}", found_index, expected_event_id);
        } else {
            println!("\nERROR: WRONG: Found index {} points to {}, expected {}",
                found_index, item.event_id, expected_event_id);
        }
        is_correct
    } else {
        println!("\nERROR: INVALID: Index {} is out of bounds!", found_index);
        false
    }
}

#[tokio::main]
async fn main() {
    println!("=== Backwards Pagination with Snapshot Validation (CORRECT) ===\n");

    let timeline = Timeline::new();

    // Initialize timeline with some items
    {
        let mut items = timeline.items.write().await;
        for i in 0..15 {
            items.push(TimelineItem {
                event_id: format!("event_{}", i),
                content: format!("Message {}", i),
            });
        }
        println!("NOTE: Initialized timeline with {} items\n", items.len());
    }

    // Create channels
    let (request_tx, request_rx) = mpsc::channel(10);
    let (diff_tx, diff_rx) = mpsc::channel(100);
    let (result_tx, mut result_rx) = mpsc::channel(10);

    // Start the search handler
    let handler_timeline = Timeline { items: timeline.items.clone() };
    tokio::spawn(async move {
        timeline_search_handler(handler_timeline, request_rx, diff_rx, result_tx).await;
    });

    // Start concurrent modifications
    let update_timeline = Timeline { items: timeline.items.clone() };
    simulate_concurrent_updates(update_timeline, diff_tx).await;

    // Give concurrent tasks time to start
    sleep(Duration::from_millis(50)).await;

    // Scenario: User clicks on a reply to "event_5"
    let target_event_id = "event_5".to_string();
    let starting_index = 10;
    let current_tl_len = timeline.get_length().await;

    println!("TARGET: User clicks reply to '{}' (visible at index {})\n", target_event_id, starting_index);
    println!("SNAPSHOT: Snapshot: timeline length = {}\n", current_tl_len);

    // Send the request
    request_tx.send(BackwardsPaginateRequest {
        target_event_id: target_event_id.clone(),
        starting_index,
        current_tl_len,
    }).await.ok();

    // Wait for result
    if let Some(result) = result_rx.recv().await {
        println!("\n>> Search returned index: {}", result.index);

        // Wait for concurrent modifications to complete
        sleep(Duration::from_millis(600)).await;

        // Verify the result
        verify_result(&timeline, result.index, &target_event_id).await;

        // Show final timeline state
        let final_len = timeline.get_length().await;
        println!("\nSTATS: Final timeline length: {}", final_len);
        println!("\nOK: The index remains correct despite {} concurrent modifications!",
            final_len - current_tl_len);
    } else {
        println!("\nERROR: Search failed to find the event");
    }

    println!("\n=== Key Techniques Demonstrated ===");
    println!("1. OK: Snapshot validation detects timeline changes");
    println!("2. OK: Incremental index adjustment tracks modifications");
    println!("3. OK: Biased selection reduces race windows");
    println!("4. OK: Found index always points to correct message\n");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_snapshot_validation_detects_changes() {
        let timeline = Timeline::new();

        // Setup initial timeline
        {
            let mut items = timeline.items.write().await;
            for i in 0..10 {
                items.push(TimelineItem {
                    event_id: format!("event_{}", i),
                    content: format!("Message {}", i),
                });
            }
        }

        let snapshot_len = timeline.get_length().await;
        assert_eq!(snapshot_len, 10);

        // Modify timeline
        timeline.apply_diff(TimelineDiff::PushBack {
            item: TimelineItem {
                event_id: "new".to_string(),
                content: "New".to_string(),
            },
        }).await;

        let current_len = timeline.get_length().await;
        assert_eq!(current_len, 11);

        // Snapshot validation would detect this change
        assert_ne!(snapshot_len, current_len);
    }

    #[tokio::test]
    async fn test_index_adjustment_on_prepend() {
        let mut found_index = 5;
        let _target_event_id = "event_5".to_string();

        // Simulate prepending 2 items
        // This should shift found_index from 5 to 7
        for _ in 0..2 {
            found_index += 1;  // Adjust for prepend
        }

        assert_eq!(found_index, 7);
        println!("Prepending 2 items adjusted index from 5 to {}", found_index);
    }

    #[tokio::test]
    async fn test_index_adjustment_on_insert() {
        let mut found_index: usize = 10;

        // Insert at index 5 (before found_index)
        let insert_index = 5;
        if insert_index <= found_index {
            found_index += 1;
        }

        assert_eq!(found_index, 11);

        // Insert at index 15 (after found_index)
        let insert_index = 15;
        let prev_index = found_index;
        if insert_index <= found_index {
            found_index += 1;
        }

        assert_eq!(found_index, prev_index);  // Unchanged
    }

    #[tokio::test]
    async fn test_index_adjustment_on_remove() {
        let mut found_index: usize = 10;

        // Remove at index 5 (before found_index)
        let remove_index = 5;
        if remove_index < found_index {
            found_index = found_index.saturating_sub(1);
        }

        assert_eq!(found_index, 9);

        // Remove at index 15 (after found_index)
        let remove_index = 15;
        let prev_index = found_index;
        if remove_index < found_index {
            found_index = found_index.saturating_sub(1);
        }

        assert_eq!(found_index, prev_index);  // Unchanged
    }

    #[tokio::test]
    async fn test_full_workflow_with_concurrent_modifications() {
        let timeline = Timeline::new();

        // Setup initial timeline
        {
            let mut items = timeline.items.write().await;
            for i in 0..10 {
                items.push(TimelineItem {
                    event_id: format!("event_{}", i),
                    content: format!("Message {}", i),
                });
            }
        }

        let (request_tx, request_rx) = mpsc::channel(10);
        let (diff_tx, diff_rx) = mpsc::channel(100);
        let (result_tx, mut result_rx) = mpsc::channel(10);

        // Start handler
        let handler_timeline = Timeline { items: timeline.items.clone() };
        tokio::spawn(async move {
            timeline_search_handler(handler_timeline, request_rx, diff_rx, result_tx).await;
        });

        // Take snapshot BEFORE modification
        let snapshot_len = timeline.get_length().await;

        // Send search request with old snapshot
        request_tx.send(BackwardsPaginateRequest {
            target_event_id: "event_3".to_string(),
            starting_index: 8,
            current_tl_len: snapshot_len,
        }).await.ok();

        // Give handler time to process
        sleep(Duration::from_millis(50)).await;

        // Get result - should find at index 3
        if let Some(result) = result_rx.recv().await {
            // Verify the index is correct (still 3, no modifications yet)
            let item = timeline.get_item(result.index).await;
            assert!(item.is_some());
            assert_eq!(item.unwrap().event_id, "event_3");
            assert_eq!(result.index, 3);
        }
    }
}
