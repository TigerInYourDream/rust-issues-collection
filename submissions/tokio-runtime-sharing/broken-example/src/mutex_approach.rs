//! Demonstrates the problem with Mutex<Option<Runtime>> approach
//!
//! PROBLEM: Mutex ensures unique access and allows controlled shutdown via take(),
//! but it's difficult to use in TSP and other long-lived tasks.
//! You can't hold a MutexGuard for a long time, and locking repeatedly is inefficient.

use std::sync::Mutex;
use std::time::Duration;
use tokio::runtime::Runtime;

static TOKIO_RUNTIME: Mutex<Option<Runtime>> = Mutex::new(None);

/// Simulates a time-series processing (TSP) task
struct TspWorker;

impl TspWorker {
    fn start_processing(&self) {
        log::info!("TSP worker attempting to start...");

        // PROBLEM 1: We need to lock the Mutex every time we want to use the runtime
        let rt_guard = TOKIO_RUNTIME.lock().unwrap();

        if let Some(rt) = rt_guard.as_ref() {
            // PROBLEM 2: We can spawn tasks, but the guard is locked during this time
            // If TSP needs long-running access, we're blocking other parts of the app!
            for i in 0..3 {
                rt.spawn(async move {
                    loop {
                        tokio::time::sleep(Duration::from_millis(500)).await;
                        log::debug!("TSP task {} processing...", i);
                    }
                });
            }
        }

        // PROBLEM 3: We have to drop the guard immediately, can't keep a reference
        drop(rt_guard);

        log::warn!("⚠️  TSP worker had to release the lock immediately");
        log::warn!("   Cannot keep a long-lived reference to the runtime");
    }

    fn do_continuous_work(&self) {
        log::info!("Attempting continuous work with repeated locking...");

        // PROBLEM 4: For continuous work, we need to lock repeatedly - inefficient!
        for i in 0..5 {
            let rt_guard = TOKIO_RUNTIME.lock().unwrap();
            if let Some(rt) = rt_guard.as_ref() {
                rt.block_on(async {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    log::debug!("Work iteration {} (had to lock Mutex again)", i);
                });
            }
            drop(rt_guard);
        }

        log::warn!("⚠️  Had to lock/unlock Mutex 5 times for continuous work");
    }
}

pub fn demonstrate_problem() {
    // Initialize the runtime
    *TOKIO_RUNTIME.lock().unwrap() = Some(Runtime::new().unwrap());
    log::info!("Created Mutex<Option<Runtime>>");

    let tsp_worker = TspWorker;

    // Show the difficulties of using Mutex approach
    tsp_worker.start_processing();

    std::thread::sleep(Duration::from_millis(100));

    tsp_worker.do_continuous_work();

    // The GOOD part: We CAN shutdown controllably
    log::info!("✅ Good: Mutex allows controlled shutdown via take()");
    let mut rt_guard = TOKIO_RUNTIME.lock().unwrap();
    if let Some(rt) = rt_guard.take() {
        log::info!("Successfully took ownership of runtime for shutdown");
        rt.shutdown_background();
    }
    drop(rt_guard);

    log::error!("❌ Mutex approach problems:");
    log::error!("   - Cannot hold long-lived reference to runtime");
    log::error!("   - Must lock/unlock repeatedly for continuous work");
    log::error!("   - Holding MutexGuard blocks other components");
    log::error!("   - Complex to use, easy to cause deadlocks");
}
