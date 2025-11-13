//! Demonstrates the PROBLEMATIC logout approach
//!
//! This module shows what happens when you:
//! 1. Drop the client (which starts destructing async tasks)
//! 2. Immediately shutdown the tokio runtime
//! 3. Async tasks are still running and panic when runtime is gone
//!
//! In a real scenario with deadpool-runtime, this would cause:
//! "thread 'main' panicked at deadpool-runtime-0.1.4/src/lib.rs:101:22:
//!  there is no reactor running, must be called from the context of a Tokio 1.x runtime"

use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::runtime::Runtime;


/// Simulates a Matrix SDK client with long-running async tasks
struct MockMatrixClient {
    // Simulates deadpool connection pool tasks
    _background_tasks: Vec<tokio::task::JoinHandle<()>>,
}

impl MockMatrixClient {
    fn new(runtime: &Runtime) -> Self {
        let mut tasks = Vec::new();

        // Spawn several background tasks that simulate deadpool-runtime behavior
        for i in 0..5 {
            let handle = runtime.spawn(async move {
                loop {
                    // Simulate periodic database connection pool maintenance
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    log::debug!("Background task {} still running", i);
                }
            });
            tasks.push(handle);
        }

        Self {
            _background_tasks: tasks,
        }
    }

    async fn logout(&self) {
        log::info!("Performing server-side logout...");
        tokio::time::sleep(Duration::from_millis(50)).await;
        log::info!("Server logout complete");
    }
}

// Global client storage (simulating static CLIENT in robrix)
static CLIENT: Mutex<Option<MockMatrixClient>> = Mutex::new(None);
static TOKIO_RUNTIME: Mutex<Option<Runtime>> = Mutex::new(None);

fn initialize_runtime() {
    let rt = Runtime::new().unwrap();
    *TOKIO_RUNTIME.lock().unwrap() = Some(rt);
}

fn get_runtime() -> Arc<Runtime> {
    // In real code, this would return a reference to the static runtime
    // For this demo, we create a temporary one
    Arc::new(Runtime::new().unwrap())
}

/// PROBLEMATIC APPROACH: Logout without proper async task cleanup
async fn problematic_logout() -> anyhow::Result<()> {
    log::info!("=== Starting PROBLEMATIC logout ===");

    let client = CLIENT.lock().unwrap().take();
    if client.is_none() {
        return Err(anyhow::anyhow!("No client found"));
    }
    let client = client.unwrap();

    // Step 1: Server logout
    log::info!("Step 1: Logging out from server...");
    client.logout().await;

    // Step 2: Drop the client immediately
    log::info!("Step 2: Dropping client (this starts async task destruction)...");
    drop(client);

    // PROBLEM: We immediately shutdown the runtime!
    // The client's background tasks are still alive and will panic
    log::warn!("Step 3: IMMEDIATELY shutting down runtime (DANGEROUS!)");

    let mut rt_guard = TOKIO_RUNTIME.lock().unwrap();
    if let Some(rt) = rt_guard.take() {
        // shutdown_background() does NOT wait for tasks to complete!
        rt.shutdown_background();
        log::error!("Runtime shut down - any remaining async tasks will panic!");
    }

    // In a real scenario with deadpool-runtime, the panic would occur here
    // because deadpool tasks try to access the now-closed runtime

    log::info!("Step 4: Attempting to restart runtime...");
    initialize_runtime();

    log::warn!("=== Logout complete (but likely caused panics in real scenario) ===");
    Ok(())
}

fn setup_client() {
    initialize_runtime();

    let rt = TOKIO_RUNTIME.lock().unwrap();
    let rt_ref = rt.as_ref().unwrap();

    let client = MockMatrixClient::new(rt_ref);
    *CLIENT.lock().unwrap() = Some(client);

    log::info!("Mock Matrix client initialized with background tasks");
}

fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    println!("\n🔴 PROBLEMATIC APPROACH DEMONSTRATION");
    println!("=====================================\n");
    println!("This demonstrates what happens when you shutdown the runtime");
    println!("immediately after dropping the client.\n");
    println!("In a real scenario with Matrix SDK and deadpool-runtime,");
    println!("this would cause a panic like:");
    println!("  'there is no reactor running, must be called from");
    println!("   the context of a Tokio 1.x runtime'\n");

    setup_client();

    // Give background tasks time to start
    std::thread::sleep(Duration::from_millis(200));

    // Perform problematic logout
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        if let Err(e) = problematic_logout().await {
            log::error!("Logout failed: {}", e);
        }
    });

    println!("\n⚠️  In a real scenario with deadpool-runtime, the program would");
    println!("have panicked during or after runtime shutdown.\n");
    println!("The issue: shutdown_background() doesn't wait for async tasks,");
    println!("causing a race condition where tasks try to use a closed runtime.\n");
}
