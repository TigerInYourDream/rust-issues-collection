// CORRECT EXAMPLE: Proper use of Pin for self-referential structures
// This code is safe and demonstrates best practices

mod pin_basics;
mod async_buf_reader;
mod alternative_designs;

fn main() {
    println!("=== Pin and Self-Referential Structures - CORRECT EXAMPLES ===\n");

    // Example 1: Pin basics
    println!("Example 1: Pin Basics");
    pin_basics::demonstrate_pin_basics();
    println!();

    // Example 2: Alternative designs (avoiding self-reference)
    println!("Example 2: Alternative Designs");
    alternative_designs::demonstrate_alternatives();
    println!();

    // Example 3: Async buffer reader (requires tokio runtime)
    println!("Example 3: Async Buffer Reader with Pin");
    println!("  (See async_buf_reader.rs and run tests with `cargo test`)");
    println!();

    println!("=== All examples completed successfully ===");
    println!("\nKey takeaways:");
    println!("1. Pin prevents moving of !Unpin types");
    println!("2. Use pin_project for safe field access");
    println!("3. PhantomPinned marks types as !Unpin");
    println!("4. Always prefer avoiding self-reference if possible");
    println!("\nRun tests:");
    println!("  cargo test");
    println!("  cargo +nightly miri test");
}
