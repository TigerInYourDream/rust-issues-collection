/// RefCell Safe Usage - Simple Solutions
///
/// This demonstrates three simple solutions to avoid RefCell panics:
/// 1. Clone and Release - Copy data before processing
/// 2. Do Everything in One Borrow - Avoid function calls
/// 3. Use Cell for Simple Types - No borrowing needed
use std::cell::{Cell, RefCell};

// ============================================================================
// Solution 1: Clone and Release (Simplest!)
// ============================================================================

thread_local! {
    static CACHE: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn add_to_cache(item: String) {
    CACHE.with(|cache| {
        cache.borrow_mut().push(item.clone());
        println!("Added: {}", item);
    });
}

/// Process items - CORRECT VERSION
///
/// SOLUTION: Clone the data first, release the borrow, then process
fn process_items_correct() {
    println!("\n[CORRECT] Processing items...");

    // Step 1: Clone the data (short-lived borrow)
    let items = CACHE.with(|cache| cache.borrow().clone());
    // Borrow is released here!

    // Step 2: Now we can safely modify CACHE while processing
    for item in items {
        println!("Processing: {}", item);

        if item.contains("special") {
            add_to_cache(format!("derived-{}", item));  // ✅ Safe!
        }
    }
}

// ============================================================================
// Solution 2: Do Everything in One Borrow
// ============================================================================

thread_local! {
    static COUNTER: RefCell<i32> = const { RefCell::new(0) };
}

/// Update and log - CORRECT VERSION
///
/// SOLUTION: Don't call other functions, do everything in one borrow
fn update_and_log_correct() {
    println!("\n[CORRECT] Update and log...");

    COUNTER.with(|c| {
        let mut counter = c.borrow_mut();
        *counter += 1;

        // Do everything within the same borrow scope
        println!("New value: {}", *counter);

        // If we need more complex logic, keep it here
        if *counter > 5 {
            println!("Counter is getting big!");
        }
    }); // Borrow released at the end
}

// ============================================================================
// Solution 3: Use Cell for Simple Types (Best for Scalars!)
// ============================================================================

/// Counter using Cell - No borrowing needed!
struct SimpleCounter {
    value: Cell<i32>,
}

impl SimpleCounter {
    fn new() -> Self {
        Self {
            value: Cell::new(0),
        }
    }

    /// Increment - no borrow needed!
    fn increment(&self) {
        self.value.set(self.value.get() + 1);
    }

    /// Get value - no borrow needed!
    fn get(&self) -> i32 {
        self.value.get()
    }

    /// Update and log - works perfectly!
    fn update_and_log(&self) {
        // Can call methods without any borrow conflicts
        self.increment();
        let value = self.get();  // ✅ No panic!
        println!("Counter: {}", value);

        if value > 5 {
            println!("Counter is getting big!");
        }
    }
}

// ============================================================================
// Main: Demonstration
// ============================================================================

fn main() {
    println!("=== RefCell Safe Usage - Simple Solutions ===\n");

    // Solution 1: Clone and Release
    println!("--- Solution 1: Clone and Release ---");
    CACHE.with(|c| c.borrow_mut().clear());

    add_to_cache("apple".to_string());
    add_to_cache("special-banana".to_string());
    add_to_cache("cherry".to_string());

    process_items_correct();

    println!("\nWhy it works:");
    println!("  - Clone creates a copy of the data");
    println!("  - Original borrow is released immediately");
    println!("  - Now we can modify the original safely");
    println!("\n  Trade-off: Uses more memory (clone cost)");

    // Solution 2: Single Borrow
    println!("\n--- Solution 2: Do Everything in One Borrow ---");
    COUNTER.with(|c| *c.borrow_mut() = 0);

    update_and_log_correct();
    update_and_log_correct();

    println!("\nWhy it works:");
    println!("  - All operations happen in one borrow scope");
    println!("  - No function calls that might re-borrow");
    println!("  - Borrow is released at the end");

    // Solution 3: Cell
    println!("\n--- Solution 3: Use Cell for Simple Types ---");
    let counter = SimpleCounter::new();

    for i in 1..=7 {
        println!("\nIteration {}", i);
        counter.update_and_log();
    }

    println!("\nWhy it works:");
    println!("  - Cell doesn't use borrowing at all!");
    println!("  - Just copies values in and out");
    println!("  - Zero overhead, no panic possible");
    println!("\n  Limitation: Only works with Copy types (i32, bool, etc)");

    println!("\n=== Summary ===");
    println!("Three simple solutions:");
    println!("  1. Clone data → Release borrow → Process");
    println!("  2. Do everything in one borrow scope");
    println!("  3. Use Cell for simple Copy types");
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clone_and_release() {
        CACHE.with(|c| c.borrow_mut().clear());

        add_to_cache("test".to_string());
        add_to_cache("special-item".to_string());

        // This should NOT panic
        process_items_correct();

        // Verify derived item was added
        let items = CACHE.with(|c| c.borrow().clone());
        assert!(items.iter().any(|s| s.contains("derived")));
    }

    #[test]
    fn test_single_borrow() {
        COUNTER.with(|c| *c.borrow_mut() = 0);

        // Should not panic
        update_and_log_correct();

        let value = COUNTER.with(|c| *c.borrow());
        assert_eq!(value, 1);
    }

    #[test]
    fn test_cell_counter() {
        let counter = SimpleCounter::new();

        assert_eq!(counter.get(), 0);

        counter.increment();
        counter.increment();
        counter.update_and_log();  // Should not panic

        assert_eq!(counter.get(), 3);
    }

    #[test]
    fn test_cell_no_panic_on_nested_calls() {
        let counter = SimpleCounter::new();

        // This pattern would panic with RefCell, but works with Cell
        for _ in 0..10 {
            counter.increment();
            let _ = counter.get();  // ✅ No panic!
            counter.update_and_log();  // ✅ No panic!
        }

        assert_eq!(counter.get(), 20);  // 10 increments + 10 from update_and_log
    }
}
