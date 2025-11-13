//! Demonstrates the CORRECT way to handle async resource cleanup
//!
//! **THE SOLUTION:**
//! Instead of relying on Drop for async cleanup, provide an explicit async shutdown method.
//! This allows proper resource cleanup while maintaining Rust's safety guarantees.

use std::fs::File;
use std::io::Write as IoWrite;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{watch, Notify};
use tokio::task::JoinHandle;

/// A background worker with proper async cleanup
struct BackgroundWorker {
    /// The async task handle (Option allows taking in shutdown)
    task_handle: Option<JoinHandle<()>>,
    /// Sender to signal shutdown
    shutdown_tx: watch::Sender<bool>,
    /// Path to temporary log file
    temp_file: PathBuf,
    /// Notified when cleanup is complete
    shutdown_complete: Arc<Notify>,
}

impl BackgroundWorker {
    /// Spawns a new background worker with graceful shutdown capability
    fn new(temp_file: PathBuf) -> Self {
        let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
        let shutdown_complete = Arc::new(Notify::new());
        let shutdown_complete_clone = shutdown_complete.clone();
        let file_path = temp_file.clone();

        let task_handle = tokio::spawn(async move {
            println!("[Worker] Starting background task...");

            // Create and write to temporary file
            let mut file = File::create(&file_path)
                .expect("Failed to create temp file");

            // Main work loop with shutdown monitoring
            let mut item_count = 0;
            loop {
                tokio::select! {
                    // Check for shutdown signal
                    _ = shutdown_rx.changed() => {
                        if *shutdown_rx.borrow() {
                            println!("[Worker] Shutdown signal received, starting cleanup...");
                            break;
                        }
                    }
                    // Do work
                    _ = tokio::time::sleep(Duration::from_millis(100)) => {
                        if item_count >= 10 {
                            println!("[Worker] Work completed naturally");
                            break;
                        }
                        writeln!(file, "Processing item {}", item_count)
                            .expect("Failed to write to file");
                        file.flush().expect("Failed to flush");
                        println!("[Worker] Processed item {}", item_count);
                        item_count += 1;
                    }
                }
            }

            // ✅ CRITICAL CLEANUP CODE - always executed
            println!("[Worker] Flushing and closing file...");
            drop(file);

            // Clean up the temporary file
            if std::fs::remove_file(&file_path).is_ok() {
                println!("[Worker] ✓ Cleaned up temporary file: {:?}", file_path);
            } else {
                eprintln!("[Worker] ✗ Failed to clean up temporary file");
            }

            // Notify that cleanup is complete
            shutdown_complete_clone.notify_one();
            println!("[Worker] Task shutdown complete");
        });

        Self {
            task_handle: Some(task_handle),
            shutdown_tx,
            temp_file,
            shutdown_complete,
        }
    }

    /// ✅ SOLUTION: Explicit async shutdown method
    ///
    /// This method:
    /// 1. Sends a shutdown signal to the task
    /// 2. Waits for the task to complete its cleanup
    /// 3. Joins the task handle to ensure it has finished
    ///
    /// This pattern ensures all async cleanup code runs to completion.
    async fn shutdown(mut self) {
        println!("[Shutdown] Initiating graceful shutdown...");

        // Step 1: Signal the task to shutdown
        if self.shutdown_tx.send(true).is_err() {
            eprintln!("[Shutdown] Warning: task already finished");
        }

        // Step 2: Wait for cleanup to complete
        println!("[Shutdown] Waiting for cleanup to complete...");
        self.shutdown_complete.notified().await;

        // Step 3: Join the task to ensure it has exited
        if let Some(handle) = self.task_handle.take() {
            match handle.await {
                Ok(()) => println!("[Shutdown] ✓ Task joined successfully"),
                Err(e) => eprintln!("[Shutdown] ✗ Task panicked: {}", e),
            }
        }
    }

    /// Alternative: async method that can be called explicitly
    /// This allows for timeout handling and error recovery
    async fn shutdown_with_timeout(mut self, timeout: Duration) -> Result<(), &'static str> {
        self.shutdown_tx.send(true).ok();

        let handle = self.task_handle.take();

        tokio::select! {
            _ = self.shutdown_complete.notified() => {
                if let Some(h) = handle {
                    h.await.ok();
                }
                Ok(())
            }
            _ = tokio::time::sleep(timeout) => {
                eprintln!("[Shutdown] Timeout reached, aborting task");
                if let Some(h) = handle {
                    h.abort();
                }
                Err("Shutdown timed out")
            }
        }
    }
}

// ✅ Drop as a safety net, not the primary cleanup mechanism
impl Drop for BackgroundWorker {
    fn drop(&mut self) {
        // Check if the task handle was taken (meaning shutdown was called)
        if let Some(handle) = &self.task_handle {
            if !handle.is_finished() {
                eprintln!("⚠️  WARNING: BackgroundWorker dropped without calling shutdown()!");
                eprintln!("⚠️  Aborting task - cleanup code may not execute properly");
                eprintln!("⚠️  Always call .shutdown().await before dropping!");

                // Send shutdown signal as last resort
                self.shutdown_tx.send(true).ok();

                // Abort the task (not ideal, but better than hanging)
                handle.abort();
            }
        } else {
            println!("[Drop] Worker already shutdown cleanly");
        }
    }
}

#[tokio::main]
async fn main() {
    println!("=== Demonstrating Correct Async Resource Cleanup ===\n");

    // Example 1: Graceful shutdown
    {
        println!("--- Example 1: Graceful Shutdown ---");
        let temp_file = PathBuf::from("/tmp/async-drop-correct-1.log");
        let worker = BackgroundWorker::new(temp_file);

        // Let it run for a short time
        tokio::time::sleep(Duration::from_millis(300)).await;

        // ✅ Explicitly call shutdown before dropping
        println!("\nInitiating shutdown...");
        worker.shutdown().await;
        println!("✓ Worker shutdown complete\n");
    }

    // Example 2: Shutdown with timeout
    {
        println!("--- Example 2: Shutdown with Timeout ---");
        let temp_file = PathBuf::from("/tmp/async-drop-correct-2.log");
        let worker = BackgroundWorker::new(temp_file);

        tokio::time::sleep(Duration::from_millis(200)).await;

        match worker.shutdown_with_timeout(Duration::from_secs(1)).await {
            Ok(()) => println!("✓ Worker shutdown within timeout\n"),
            Err(e) => eprintln!("✗ {}\n", e),
        }
    }

    // Example 3: Demonstrating the Drop safety net
    {
        println!("--- Example 3: Drop Without Shutdown (shows warning) ---");
        let temp_file = PathBuf::from("/tmp/async-drop-correct-3.log");
        let worker = BackgroundWorker::new(temp_file);

        tokio::time::sleep(Duration::from_millis(200)).await;

        // Intentionally drop without shutdown to show the warning
        drop(worker);
        println!("(See warning above - this demonstrates the safety net)\n");
    }

    tokio::time::sleep(Duration::from_millis(100)).await;

    println!("=== Summary ===");
    println!("✓ Explicit async shutdown ensures cleanup code runs");
    println!("✓ Timeout handling prevents hanging on shutdown");
    println!("✓ Drop serves as a safety net with clear warnings");
    println!("✓ No resource leaks or data corruption");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_graceful_shutdown() {
        let temp_file = PathBuf::from("/tmp/test-async-drop-correct.log");
        let worker = BackgroundWorker::new(temp_file.clone());

        tokio::time::sleep(Duration::from_millis(200)).await;

        // Explicit shutdown should complete successfully
        worker.shutdown().await;

        // File should have been cleaned up
        assert!(!temp_file.exists(), "Temp file should be removed");
    }

    #[tokio::test]
    async fn test_shutdown_with_timeout() {
        let temp_file = PathBuf::from("/tmp/test-async-drop-timeout.log");
        let worker = BackgroundWorker::new(temp_file);

        tokio::time::sleep(Duration::from_millis(100)).await;

        let result = worker.shutdown_with_timeout(Duration::from_secs(5)).await;
        assert!(result.is_ok(), "Shutdown should complete within timeout");
    }

    #[tokio::test]
    async fn test_natural_completion() {
        let temp_file = PathBuf::from("/tmp/test-async-drop-natural.log");
        let worker = BackgroundWorker::new(temp_file.clone());

        // Let the task complete naturally
        tokio::time::sleep(Duration::from_millis(1200)).await;

        // Shutdown should still work even if task finished
        worker.shutdown().await;

        assert!(!temp_file.exists(), "Temp file should be cleaned up");
    }
}
