//! Demonstrates the problem with Arc<Runtime> approach
//!
//! PROBLEM: Arc allows easy sharing, but you cannot force all references to drop.
//! When shutdown is needed, you have no way to ensure all Arc clones are released,
//! leading to potential resource leaks or dangling runtime references.

use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;

/// Simulates a time-series processing (TSP) task that needs long-lived runtime access
struct TspWorker {
    runtime: Arc<Runtime>,
}

impl TspWorker {
    fn new(runtime: Arc<Runtime>) -> Self {
        Self { runtime }
    }

    fn start_processing(&self) {
        // TSP spawns background tasks that may run for a long time
        for i in 0..3 {
            self.runtime.spawn(async move {
                loop {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    log::debug!("TSP task {} processing...", i);
                }
            });
        }
        log::info!("TSP worker started with {} background tasks", 3);
    }
}

pub fn demonstrate_problem() {
    let runtime = Arc::new(Runtime::new().unwrap());
    log::info!("Created Arc<Runtime>");

    // Scenario 1: TSP worker clones the Arc
    let tsp_worker = TspWorker::new(Arc::clone(&runtime));
    tsp_worker.start_processing();

    // Scenario 2: Multiple components might clone the runtime
    let runtime_clone1 = Arc::clone(&runtime);
    let runtime_clone2 = Arc::clone(&runtime);

    log::info!("Runtime Arc reference count: {}", Arc::strong_count(&runtime));

    // Simulate some work
    std::thread::sleep(Duration::from_millis(200));

    // Now we want to shutdown and restart the runtime (e.g., during logout)
    log::warn!("⚠️  PROBLEM: Trying to shutdown, but Arc references still exist!");
    log::warn!("   Arc strong count: {}", Arc::strong_count(&runtime));

    // Drop our local references
    drop(runtime_clone1);
    drop(runtime_clone2);
    drop(tsp_worker);

    log::warn!("   Even after dropping local refs, main Arc still exists");
    log::warn!("   We cannot force shutdown because Arc doesn't provide that control");

    // The only way to shutdown is to drop ALL Arc references
    // But in a real app, you don't know where all the clones are!
    drop(runtime);

    log::error!("❌ Arc approach problem:");
    log::error!("   - Cannot actively terminate all Arc clones");
    log::error!("   - No way to ensure timely shutdown");
    log::error!("   - Risk of dangling Arc references after shutdown attempt");
}
