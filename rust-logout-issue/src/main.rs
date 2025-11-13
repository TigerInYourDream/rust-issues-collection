//! Tokio Runtime Shutdown Issue with Async Tasks
//!
//! This project demonstrates a critical issue that occurs when shutting down
//! a tokio runtime while async tasks (particularly deadpool-runtime tasks) are still running.
//!
//! # Problem
//! When a Matrix SDK client is dropped and the tokio runtime is shut down immediately,
//! internal async tasks (from deadpool-runtime used by rusqlite) may still be running.
//! These tasks panic when they try to execute after the runtime has been closed.
//!
//! # Solution
//! Implement a state machine for the logout process that ensures proper ordering:
//! 1. Stop sync service
//! 2. Perform server logout
//! 3. Clean app state (drop client)
//! 4. Wait for cleanup confirmation (gives time for async tasks to complete)
//! 5. Shutdown background tasks
//! 6. Restart runtime
//!
//! # Usage
//! Run the problem demonstration:
//! ```bash
//! cargo run --bin problem
//! ```
//!
//! Run the solution demonstration:
//! ```bash
//! cargo run --bin solution
//! ```

fn main() {
    println!("Tokio Runtime Shutdown Issue Demonstration");
    println!("===========================================\n");
    println!("This project demonstrates the deadpool-runtime panic issue");
    println!("that occurs when shutting down tokio runtime prematurely.\n");
    println!("Run one of the following binaries:");
    println!("  cargo run --bin problem   - Shows the problematic approach");
    println!("  cargo run --bin solution  - Shows the state machine solution\n");
    println!("Run tests with:");
    println!("  cargo test");
}
