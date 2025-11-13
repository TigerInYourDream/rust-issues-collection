//! Demonstrates the async resource Drop trap
//!
//! **THE PROBLEM:**
//! The Drop trait in Rust is synchronous, but async resources need async cleanup.
//! This example shows what goes WRONG when you naively handle async resource cleanup in Drop.

use std::fs::File;
use std::io::Write as IoWrite;
use std::path::PathBuf;
use std::time::Duration;
use tokio::task::JoinHandle;

/// A background worker that processes data and writes to a log file
struct BackgroundWorker {
    /// The async task handle
    task_handle: JoinHandle<()>,
    /// Path to temporary log file that should be cleaned up
    temp_file: PathBuf,
}

impl BackgroundWorker {
    /// Spawns a new background worker
    fn new(temp_file: PathBuf) -> Self {
        let file_path = temp_file.clone();

        let task_handle = tokio::spawn(async move {
            println!("[Worker] Starting background task...");

            // Create and write to temporary file
            let mut file = File::create(&file_path)
                .expect("Failed to create temp file");

            // Simulate ongoing work with periodic writes
            for i in 0..10 {
                tokio::time::sleep(Duration::from_millis(100)).await;
                writeln!(file, "Processing item {}", i)
                    .expect("Failed to write to file");
                file.flush().expect("Failed to flush");
                println!("[Worker] Processed item {}", i);
            }

            // ⚠️ CRITICAL CLEANUP CODE that should run before exit
            println!("[Worker] Task finishing, cleaning up resources...");
            drop(file);

            // Clean up the temporary file
            if std::fs::remove_file(&file_path).is_ok() {
                println!("[Worker] ✓ Cleaned up temporary file: {:?}", file_path);
            }

            println!("[Worker] Task completed gracefully");
        });

        Self {
            task_handle,
            temp_file,
        }
    }
}

// ❌ PROBLEM: Synchronous Drop with async resources
impl Drop for BackgroundWorker {
    fn drop(&mut self) {
        println!("[Drop] Dropping BackgroundWorker...");

        // ❌ ISSUE 1: abort() immediately terminates the task
        // The cleanup code in the task (file deletion) will NEVER run!
        self.task_handle.abort();
        println!("[Drop] ✗ Aborted task (cleanup code was not executed!)");

        // ❌ ISSUE 2: Cannot use .await in Drop to wait for task completion
        // This would require async fn drop(), which doesn't exist
        // Uncommenting this would cause a compile error:
        // self.task_handle.await.ok();

        // ❌ ISSUE 3: Using block_on in Drop can cause deadlock
        // If Drop is called from an async context, this will panic or deadlock:
        // tokio::runtime::Handle::current()
        //     .block_on(async { self.task_handle.await.ok() });

        // ❌ ISSUE 4: Manual cleanup in Drop is unreliable
        // The file might still be locked by the aborted task
        if std::fs::remove_file(&self.temp_file).is_ok() {
            println!("[Drop] ✓ Removed temp file (but task's cleanup was skipped)");
        } else {
            println!("[Drop] ✗ Failed to remove temp file");
        }
    }
}

#[tokio::main]
async fn main() {
    println!("=== Demonstrating Async Resource Drop Trap ===\n");

    let temp_file = PathBuf::from("/tmp/async-drop-broken.log");

    {
        println!("Creating BackgroundWorker...");
        let worker = BackgroundWorker::new(temp_file.clone());

        // Let it run for a short time
        tokio::time::sleep(Duration::from_millis(300)).await;

        println!("\nDropping worker (simulating early shutdown)...");
        drop(worker);
        // ⚠️ The Drop implementation aborts the task immediately!
        // The task's cleanup code (lines 35-40) will NOT execute
    }

    // Give time to observe the consequences
    tokio::time::sleep(Duration::from_millis(100)).await;

    println!("\n=== Result ===");
    println!("✗ Task was aborted mid-execution");
    println!("✗ Task's cleanup code was NOT executed");
    println!("✗ Resources may be leaked or left in inconsistent state");
    println!("\nSee the correct-example for the proper solution!");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_demonstrates_the_problem() {
        let temp_file = PathBuf::from("/tmp/test-async-drop-broken.log");

        {
            let worker = BackgroundWorker::new(temp_file.clone());
            tokio::time::sleep(Duration::from_millis(200)).await;
            // When worker is dropped here, the task is aborted
            // and cleanup code doesn't run
        }

        // This test passes, but demonstrates the problematic behavior
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
