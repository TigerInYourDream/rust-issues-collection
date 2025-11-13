//! Broken Example: Backwards Pagination with Index Invalidation
//!
//! This example demonstrates the race condition that occurs when searching for
//! a specific event in a timeline that is being concurrently modified.
//!
//! # The Problem
//!
//! When a user clicks on a message reply to jump to the original message:
//! 1. We start searching backwards from a given index
//! 2. Meanwhile, new messages arrive (appended to end)
//! 3. Meanwhile, old messages are loaded via pagination (prepended to start)
//! 4. The index we found is NO LONGER CORRECT!
//!
//! # Run this example
//!
//! ```bash
//! cargo run --bin broken
//! ```
//!
//! You'll see that the found index is often incorrect due to concurrent modifications.

use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};

/// A simplified timeline item representing a message
#[derive(Debug, Clone)]
struct TimelineItem {
    event_id: String,
    content: String,
}

/// Timeline with concurrent read/write access
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
}

/// ERROR: BROKEN: Naive search that breaks under concurrent modifications
///
/// This function demonstrates three critical flaws:
///
/// 1. **Index Staleness**: The starting_index may already be invalid by the time
///    we start processing
/// 2. **No Modification Detection**: We cannot detect if the timeline changed
///    during our search
/// 3. **Race Window**: Between releasing the read lock and acquiring it again,
///    arbitrary modifications can occur
async fn search_for_event_broken(
    timeline: Timeline,
    target_event_id: String,
    starting_index: usize,
) -> Option<usize> {
    let mut current_index = starting_index;

    loop {
        println!("  [Search] Searching from index {}...", current_index);

        let items = timeline.items.read().await;

        // Search backwards from current_index
        for i in (0..current_index.min(items.len())).rev() {
            if let Some(item) = items.get(i) {
                if item.event_id == target_event_id {
                    println!("  [Search] OK Found target at index {}", i);
                    // ERROR: PROBLEM: This index may already be stale!
                    // Between now and when we return, timeline may change
                    return Some(i);
                }
            }
        }

        drop(items);  // Release lock

        // Not found in current timeline, need to paginate
        println!("  [Search] Not found, would trigger pagination...");
        sleep(Duration::from_millis(100)).await;

        // ERROR: PROBLEM: During this await, timeline may have changed dramatically!
        // - New messages may have arrived (appended to end)
        // - Pagination may have completed (prepended to start)
        // - Events may have been removed or modified

        // Update current_index based on new timeline length
        let new_len = timeline.get_length().await;

        if new_len == current_index {
            // No new items loaded, give up
            println!("  [Search] X Not found and no progress made");
            return None;
        }

        current_index = new_len;
    }
}

/// Simulate concurrent timeline modifications
///
/// This spawns background tasks that continuously modify the timeline:
/// - Appends new messages (simulating incoming chat messages)
/// - Prepends old messages (simulating pagination loading history)
async fn simulate_concurrent_updates(timeline: Timeline) {
    // Task 1: Simulate new messages arriving
    let timeline_clone = Timeline {
        items: timeline.items.clone(),
    };
    tokio::spawn(async move {
        for i in 0..5 {
            sleep(Duration::from_millis(80)).await;

            let mut items = timeline_clone.items.write().await;
            items.push(TimelineItem {
                event_id: format!("new_message_{}", i),
                content: format!("New message {}", i),
            });
            println!("  [Timeline] UP: New message appended, length now: {}", items.len());
        }
    });

    // Task 2: Simulate pagination loading old messages
    let timeline_clone2 = Timeline {
        items: timeline.items.clone(),
    };
    tokio::spawn(async move {
        for i in 0..5 {
            sleep(Duration::from_millis(120)).await;

            let mut items = timeline_clone2.items.write().await;
            items.insert(0, TimelineItem {
                event_id: format!("old_message_{}", i),
                content: format!("Old message {}", i),
            });
            println!("  [Timeline] DOWN: Old message prepended, length now: {}", items.len());
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
    println!("=== Backwards Pagination Index Invalidation Demo (BROKEN) ===\n");

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

    // Start concurrent modifications
    let timeline_for_updates = Timeline {
        items: timeline.items.clone(),
    };
    simulate_concurrent_updates(timeline_for_updates).await;

    // Give concurrent tasks time to start
    sleep(Duration::from_millis(50)).await;

    // Scenario: User clicks on a reply to "event_5"
    let target_event_id = "event_5".to_string();
    let starting_index = 10;

    println!("TARGET: User clicks reply to '{}' (visible at index {})\n", target_event_id, starting_index);

    // Search for the event
    let timeline_for_search = Timeline {
        items: timeline.items.clone(),
    };

    match search_for_event_broken(timeline_for_search, target_event_id.clone(), starting_index).await {
        Some(found_index) => {
            println!("\n>> Search returned index: {}", found_index);

            // Wait for concurrent modifications to complete
            sleep(Duration::from_millis(500)).await;

            // Verify the result
            let timeline_for_verify = Timeline {
                items: timeline.items.clone(),
            };
            verify_result(&timeline_for_verify, found_index, &target_event_id).await;

            // Show final timeline state
            let final_len = timeline_for_verify.get_length().await;
            println!("\nSTATS: Final timeline length: {}", final_len);
            println!("\nWARNING: The index was valid when found, but became invalid due to concurrent modifications!");
        }
        None => {
            println!("\nERROR: Search failed to find the event");
        }
    }

    println!("\n=== Key Problems Demonstrated ===");
    println!("1. RED Index becomes stale during async operations");
    println!("2. RED No detection of timeline changes");
    println!("3. RED Found index points to WRONG message");
    println!("4. RED User scrolls to incorrect position\n");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_index_invalidation() {
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

        let target_event_id = "event_3".to_string();
        let search_timeline = Timeline { items: timeline.items.clone() };

        // Start search
        let search_handle = tokio::spawn(async move {
            search_for_event_broken(search_timeline, target_event_id, 8).await
        });

        // Concurrently modify timeline
        sleep(Duration::from_millis(50)).await;
        {
            let mut items = timeline.items.write().await;
            // Prepend items - this shifts all indices forward
            items.insert(0, TimelineItem {
                event_id: "prepended_1".to_string(),
                content: "Prepended".to_string(),
            });
            items.insert(0, TimelineItem {
                event_id: "prepended_2".to_string(),
                content: "Prepended".to_string(),
            });
        }

        let found_index = search_handle.await.unwrap();

        if let Some(idx) = found_index {
            let item = timeline.get_item(idx).await;
            // The found index is likely WRONG due to prepended items
            if let Some(item) = item {
                // This assertion will likely fail, demonstrating the bug
                // The index should be idx + 2 (due to 2 prepended items)
                println!("Found item: {} at index {}", item.event_id, idx);
                println!("Expected: event_3, but may have gotten something else due to race");
            }
        }
    }

    #[tokio::test]
    async fn test_no_snapshot_validation() {
        let timeline = Timeline::new();

        {
            let mut items = timeline.items.write().await;
            for i in 0..5 {
                items.push(TimelineItem {
                    event_id: format!("event_{}", i),
                    content: format!("Message {}", i),
                });
            }
        }

        let initial_len = timeline.get_length().await;

        // Simulate timeline change
        {
            let mut items = timeline.items.write().await;
            items.push(TimelineItem {
                event_id: "new_event".to_string(),
                content: "New".to_string(),
            });
        }

        let final_len = timeline.get_length().await;

        // Demonstrate that we CAN detect changes via length
        // But the broken example doesn't use this for validation!
        assert_ne!(initial_len, final_len);
        println!("Timeline changed from {} to {} items, but broken example doesn't validate this!",
            initial_len, final_len);
    }
}
