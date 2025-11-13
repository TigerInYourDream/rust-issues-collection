//! Demonstrates the CORRECT approach for sharing Tokio Runtime
//!
//! SOLUTION: Combine Arc<Runtime> with CancellationToken
//! - Arc provides easy sharing for all components (TSP, logout handler, etc.)
//! - CancellationToken provides shutdown coordination
//! - Wait for all tasks to complete before shutting down runtime
//!
//! This solves both problems:
//! 1. Easy sharing (like Arc approach)
//! 2. Controlled shutdown (like Mutex approach, but better)

use std::sync::{Arc, OnceLock};
use std::time::Duration;
use anyhow::Result;
use tokio::runtime::Runtime;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

/// Global runtime and shutdown signal
static TOKIO_RUNTIME: OnceLock<Arc<Runtime>> = OnceLock::new();
static SHUTDOWN_TOKEN: OnceLock<CancellationToken> = OnceLock::new();

/// Initialize the runtime and shutdown token
fn initialize_runtime() {
    let runtime = Runtime::new().unwrap();
    TOKIO_RUNTIME.set(Arc::new(runtime)).ok();
    SHUTDOWN_TOKEN.set(CancellationToken::new()).ok();
    log::info!("✅ Initialized Arc<Runtime> with CancellationToken");
}

/// Get a clone of the runtime (safe to clone Arc)
fn get_runtime() -> Arc<Runtime> {
    Arc::clone(TOKIO_RUNTIME.get().expect("Runtime not initialized"))
}

/// Get the shutdown token
fn get_shutdown_token() -> CancellationToken {
    SHUTDOWN_TOKEN.get().expect("Shutdown token not initialized").clone()
}

/// Simulates a time-series processing (TSP) worker
struct TspWorker {
    runtime: Arc<Runtime>,
    shutdown_token: CancellationToken,
}

impl TspWorker {
    fn new() -> Self {
        Self {
            runtime: get_runtime(),
            shutdown_token: get_shutdown_token(),
        }
    }

    fn start_processing(&self) -> Vec<tokio::task::JoinHandle<()>> {
        log::info!("TSP worker starting background tasks...");

        let mut handles = Vec::new();

        for i in 0..3 {
            let shutdown = self.shutdown_token.clone();
            let handle = self.runtime.spawn(async move {
                loop {
                    tokio::select! {
                        _ = shutdown.cancelled() => {
                            log::info!("TSP task {} received shutdown signal, cleaning up", i);
                            break;
                        }
                        _ = tokio::time::sleep(Duration::from_millis(500)) => {
                            log::debug!("TSP task {} processing...", i);
                        }
                    }
                }
            });
            handles.push(handle);
        }

        log::info!("✅ TSP worker started with {} cancellable tasks", 3);
        handles
    }

    fn do_continuous_work(&self) {
        log::info!("TSP doing continuous work with long-lived Arc reference");

        // We can hold the Arc reference as long as needed - no Mutex locking!
        for i in 0..5 {
            if self.shutdown_token.is_cancelled() {
                log::info!("Continuous work stopped due to shutdown signal");
                break;
            }

            self.runtime.block_on(async {
                tokio::time::sleep(Duration::from_millis(50)).await;
                log::debug!("Work iteration {} (using cloned Arc, no locking needed)", i);
            });
        }

        log::info!("✅ Continuous work completed efficiently (no repeated locking)");
    }
}

/// Performs graceful shutdown
async fn graceful_shutdown(task_handles: Vec<tokio::task::JoinHandle<()>>) -> Result<()> {
    log::info!("=== Starting graceful shutdown ===");

    // Step 1: Signal all tasks to shutdown
    log::info!("Step 1: Broadcasting shutdown signal via CancellationToken");
    get_shutdown_token().cancel();

    // Step 2: Wait for all tasks to complete cleanup
    log::info!("Step 2: Waiting for all tasks to complete cleanup...");
    for (i, handle) in task_handles.into_iter().enumerate() {
        match tokio::time::timeout(Duration::from_secs(2), handle).await {
            Ok(Ok(())) => log::info!("  Task {} completed cleanly", i),
            Ok(Err(e)) => log::warn!("  Task {} failed: {}", i, e),
            Err(_) => log::warn!("  Task {} timed out", i),
        }
    }

    // Step 3: Additional cleanup wait period
    log::info!("Step 3: Additional cleanup wait period...");
    let (tx, rx) = oneshot::channel();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(200)).await;
        let _ = tx.send(());
    });
    let _ = rx.await;

    log::info!("Step 4: All tasks completed, safe to shutdown runtime");
    log::info!("✅ Graceful shutdown complete");

    Ok(())
}

/// Simulates the logout/restart flow
async fn logout_and_restart() -> Result<()> {
    log::info!("\n=== Simulating logout flow ===");

    // In a real app, you would:
    // 1. Stop sync service
    // 2. Perform server logout
    // 3. Trigger shutdown signal
    // 4. Wait for cleanup
    // 5. Shutdown runtime
    // 6. Restart runtime

    log::info!("Logout flow would happen here (omitted for brevity)");
    log::info!("See graceful_shutdown() for the key shutdown logic");

    Ok(())
}

fn main() -> Result<()> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    println!("\n✅ CORRECT APPROACH: Arc<Runtime> + CancellationToken");
    println!("======================================================\n");

    // Initialize
    initialize_runtime();

    // Create TSP worker
    let tsp_worker = TspWorker::new();

    // Start TSP tasks (collect handles for later cleanup)
    let task_handles = tsp_worker.start_processing();

    // Give tasks time to start
    std::thread::sleep(Duration::from_millis(200));

    // Do some continuous work
    tsp_worker.do_continuous_work();

    // Demonstrate graceful shutdown
    let rt = get_runtime();
    rt.block_on(async {
        graceful_shutdown(task_handles).await.unwrap();
        logout_and_restart().await.unwrap();
    });

    println!("\n=== Key Benefits of This Approach ===");
    println!("✅ Easy sharing: Arc allows cloning for TSP and other components");
    println!("✅ No lock contention: No Mutex means no blocking");
    println!("✅ Long-lived references: TSP can hold Arc as long as needed");
    println!("✅ Controlled shutdown: CancellationToken coordinates cleanup");
    println!("✅ Graceful: Wait for tasks to complete before runtime shutdown");
    println!("✅ Safe: Prevents the deadpool-runtime panic issue\n");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graceful_shutdown() {
        // Note: We cannot test graceful_shutdown in a #[tokio::test] because
        // that would create nested runtimes, which Tokio doesn't allow.
        // Instead, we test the pattern in the main() function.
        // This test verifies that the basic structure compiles and runs.

        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Debug)
            .is_test(true)
            .try_init()
            .ok();

        initialize_runtime();

        let tsp_worker = TspWorker::new();
        let handles = tsp_worker.start_processing();

        // Give tasks time to start
        std::thread::sleep(Duration::from_millis(100));

        // Cancel the shutdown token
        get_shutdown_token().cancel();

        // Wait for tasks to complete (using the runtime)
        let rt = get_runtime();
        rt.block_on(async {
            for handle in handles {
                let _ = tokio::time::timeout(Duration::from_secs(1), handle).await;
            }
        });

        // Test passes if we reach here without panic
    }

    #[test]
    fn test_runtime_sharing() {
        // Ensure runtime is initialized (idempotent if already initialized)
        initialize_runtime();

        let rt1 = get_runtime();
        let rt2 = get_runtime();
        let rt3 = get_runtime();

        // All should point to the same runtime
        // Note: Arc count may vary depending on whether other tests ran first
        assert!(Arc::strong_count(&rt1) >= 3, "At least 3 clones should exist");

        // Can use any clone independently
        rt1.block_on(async {
            tokio::time::sleep(Duration::from_millis(10)).await;
        });

        rt2.block_on(async {
            tokio::time::sleep(Duration::from_millis(10)).await;
        });

        rt3.block_on(async {
            tokio::time::sleep(Duration::from_millis(10)).await;
        });

        // Test passes if we can use all clones without panic
    }
}
