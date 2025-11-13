/// RefCell Double Borrow Panic - Simple Examples
///
/// This demonstrates two common scenarios where RefCell panics at runtime:
/// 1. Modifying a collection while iterating over it
/// 2. Calling a function that borrows while already holding a borrow
use std::cell::RefCell;

// ============================================================================
// Scenario 1: Modifying Collection During Iteration (Most Common!)
// ============================================================================

thread_local! {
    /// A simple cache that stores strings
    static CACHE: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

/// Add an item to the cache
fn add_to_cache(item: String) {
    CACHE.with(|cache| {
        cache.borrow_mut().push(item.clone());
        println!("Added: {}", item);
    });
}

/// Process items - BROKEN VERSION
///
/// PROBLEM: We borrow the cache to iterate, then try to borrow it again
/// to add new items. This causes a panic!
fn process_items_broken() {
    println!("\n[BROKEN] Processing items...");

    CACHE.with(|cache| {
        // Start borrowing here (immutable borrow)
        for item in cache.borrow().iter() {
            println!("Processing: {}", item);

            // Uncomment this line to see the panic:
            // if item.contains("special") {
            //     add_to_cache(format!("derived-{}", item));  // ❌ PANIC!
            // }
            // Error: "already borrowed: BorrowMutError"
        }
        // Borrow released here
    });
}

// ============================================================================
// Scenario 2: Function Call Chain with Same RefCell
// ============================================================================

thread_local! {
    /// Simple counter state
    static COUNTER: RefCell<i32> = const { RefCell::new(0) };
}

/// Increment the counter
fn increment() {
    COUNTER.with(|c| {
        *c.borrow_mut() += 1;
    });
}

/// Get current value
fn get_value() -> i32 {
    COUNTER.with(|c| *c.borrow())
}

/// Update and log - BROKEN VERSION
///
/// PROBLEM: We hold a mutable borrow, then try to call get_value()
/// which needs another borrow. This causes a panic!
fn update_and_log_broken() {
    println!("\n[BROKEN] Update and log...");

    COUNTER.with(|c| {
        let mut counter = c.borrow_mut();  // Mutable borrow starts
        *counter += 1;

        // Uncomment this line to see the panic:
        // let value = get_value();  // ❌ PANIC! Tries to borrow again
        // println!("New value: {}", value);
        // Error: "already borrowed: BorrowMutError"

        // We have to do it manually within the same borrow:
        println!("New value: {}", *counter);
    }); // Borrow released here
}

// ============================================================================
// Main: Demonstration
// ============================================================================

fn main() {
    println!("=== RefCell Double Borrow Panic - Simple Examples ===\n");
    println!("This demonstrates why RefCell can panic at runtime.\n");

    // Scenario 1: Collection iteration
    println!("--- Scenario 1: Iterator + Modify ---");
    add_to_cache("apple".to_string());
    add_to_cache("special-banana".to_string());
    add_to_cache("cherry".to_string());

    process_items_broken();

    println!("\nWhy it breaks:");
    println!("  - borrow() holds an immutable reference");
    println!("  - add_to_cache() needs a mutable reference");
    println!("  - Can't have both at the same time!");

    // Scenario 2: Function call chain
    println!("\n--- Scenario 2: Nested Function Calls ---");
    increment();
    println!("Counter: {}", get_value());

    update_and_log_broken();

    println!("\nWhy it breaks:");
    println!("  - borrow_mut() holds a mutable reference");
    println!("  - get_value() needs another reference");
    println!("  - Can't borrow while already borrowed!");

    println!("\n=== Key Takeaway ===");
    println!("RefCell checks borrow rules at RUNTIME, not compile-time.");
    println!("If you violate the rules, your program panics!");
    println!("\nSee correct-example for solutions.");
}

// ============================================================================
// Tests - Including panic tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_add() {
        CACHE.with(|c| c.borrow_mut().clear());
        add_to_cache("test".to_string());

        let count = CACHE.with(|c| c.borrow().len());
        assert_eq!(count, 1);
    }

    #[test]
    #[should_panic(expected = "already borrowed")]
    fn test_iterator_modify_panic() {
        // This test demonstrates the actual panic
        CACHE.with(|c| c.borrow_mut().clear());
        add_to_cache("test".to_string());

        // This WILL panic:
        CACHE.with(|cache| {
            for item in cache.borrow().iter() {
                add_to_cache(format!("new-{}", item));  // ❌ PANIC!
            }
        });
    }

    #[test]
    fn test_counter_basic() {
        COUNTER.with(|c| *c.borrow_mut() = 0);
        increment();
        assert_eq!(get_value(), 1);
    }

    #[test]
    #[should_panic(expected = "already")]
    fn test_nested_borrow_panic() {
        // This test demonstrates the actual panic
        COUNTER.with(|c| *c.borrow_mut() = 0);

        // This WILL panic:
        COUNTER.with(|c| {
            let _guard = c.borrow_mut();  // Hold mutable borrow
            let _value = get_value();     // ❌ PANIC! Try to borrow again
        });
    }
}
