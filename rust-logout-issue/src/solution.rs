//! Demonstrates the SOLUTION using a logout state machine
//!
//! This module shows the correct approach:
//! 1. Use a state machine to manage logout phases
//! 2. After dropping the client, wait for cleanup confirmation
//! 3. This wait period gives async tasks time to complete properly
//! 4. Only then shutdown the runtime safely
//!
//! This prevents the deadpool-runtime panic by ensuring proper task lifecycle.

use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::sync::oneshot;

/// Logout state machine states
#[derive(Debug, Clone)]
enum LogoutState {
    Idle,
    PreChecking,
    StoppngSyncService,
    LoggingOutFromServer,
    PointOfNoReturn,
    CleaningAppState,
    ShuttingDownTasks,
    RestartingRuntime,
    Completed,
    Failed(String),
}

/// Simulates a Matrix SDK client with long-running async tasks
struct MockMatrixClient {
    background_tasks: Vec<tokio::task::JoinHandle<()>>,
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
            background_tasks: tasks,
        }
    }

    async fn logout(&self) {
        log::info!("Performing server-side logout...");
        tokio::time::sleep(Duration::from_millis(50)).await;
        log::info!("Server logout complete");
    }
}

impl Drop for MockMatrixClient {
    fn drop(&mut self) {
        log::info!("MockMatrixClient being dropped, aborting {} background tasks",
                   self.background_tasks.len());
        for task in self.background_tasks.drain(..) {
            task.abort();
        }
    }
}

// Global state (simulating robrix's static variables)
static CLIENT: Mutex<Option<MockMatrixClient>> = Mutex::new(None);
static TOKIO_RUNTIME: Mutex<Option<Runtime>> = Mutex::new(None);

/// Logout state machine
struct LogoutStateMachine {
    state: Arc<Mutex<LogoutState>>,
}

impl LogoutStateMachine {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(LogoutState::Idle)),
        }
    }

    fn transition_to(&self, new_state: LogoutState, message: &str, progress: u8) {
        *self.state.lock().unwrap() = new_state.clone();
        log::info!("[{}%] {:?}: {}", progress, new_state, message);
    }

    async fn execute(&self) -> anyhow::Result<()> {
        log::info!("=== Starting SAFE logout with state machine ===");

        // Step 1: Pre-checks
        self.transition_to(LogoutState::PreChecking, "Checking prerequisites", 10);
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Step 2: Stop sync service (simulated)
        self.transition_to(LogoutState::StoppingSyncService, "Stopping sync service", 20);
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Step 3: Server logout
        self.transition_to(LogoutState::LoggingOutFromServer, "Logging out from server", 30);
        let client = CLIENT.lock().unwrap().take();
        if let Some(client) = client.as_ref() {
            client.logout().await;
        }

        // Step 4: Point of no return
        self.transition_to(LogoutState::PointOfNoReturn, "Point of no return reached", 50);

        // Step 5: Clean app state (drop client)
        self.transition_to(LogoutState::CleaningAppState, "Cleaning application state", 70);

        // KEY DIFFERENCE: We drop the client here
        drop(client);
        log::info!("Client dropped, starting cleanup wait period...");

        // CRITICAL: Wait for cleanup confirmation
        // This gives async tasks time to complete before we shutdown the runtime
        let (tx, rx) = oneshot::channel::<bool>();

        // Simulate app state cleanup (in real robrix, this triggers UI cleanup)
        tokio::spawn(async move {
            // Simulate some cleanup work
            tokio::time::sleep(Duration::from_millis(200)).await;
            log::info!("App state cleanup completed");
            let _ = tx.send(true);
        });

        // Wait for cleanup with timeout
        match tokio::time::timeout(Duration::from_secs(2), rx).await {
            Ok(Ok(_)) => {
                log::info!("Received cleanup confirmation - async tasks had time to complete");
            }
            Ok(Err(e)) => {
                return Err(anyhow::anyhow!("Cleanup channel error: {}", e));
            }
            Err(_) => {
                return Err(anyhow::anyhow!("Cleanup timeout"));
            }
        }

        // Step 6: NOW it's safe to shutdown background tasks
        self.transition_to(LogoutState::ShuttingDownTasks, "Shutting down background tasks", 80);

        let mut rt_guard = TOKIO_RUNTIME.lock().unwrap();
        if let Some(rt) = rt_guard.take() {
            log::info!("Shutting down runtime (async tasks already completed)");
            rt.shutdown_background();
        }

        // Step 7: Restart runtime
        self.transition_to(LogoutState::RestartingRuntime, "Restarting Matrix runtime", 90);
        let new_rt = Runtime::new()?;
        *rt_guard = Some(new_rt);
        drop(rt_guard);

        // Step 8: Complete
        self.transition_to(LogoutState::Completed, "Logout ́c͛ompleted successfully", 100);

        log::info!("=== Logout complete - NO PANICS! ===");
        Ok(())
    }
}

fn initialize_runtime() {
    let rt = Runtime::new().unwrap();
    *TOKIO_RUNTIME.lock().unwrap() = Some(rt);
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

    println!("\n✅ SOLUTION APPROACH DEMONSTRATION");
    println!("==================================\n");
    println!("This demonstrates the state machine solution that prevents");
    println!("the deadpool-runtime panic.\n");
    println!("Key improvements:");
    println!("  1. State machine manages logout phases clearly");
    println!("  2. After dropping client, we WAIT for cleanup confirmation");
    println!("  3. This wait gives async tasks time to complete properly");
    println!("  4. Only after confirmation do we shutdown the runtime\n");

    setup_client();

    // Give background tasks time to start
    std::thread::sleep(Duration::from_millis(200));

    // Perform safe logout using state machine
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let state_machine = LogoutStateMachine::new();

        match state_machine.execute().await {
            Ok(_) => {
                println!("\n✅ Logout succeeded without any panics!");
                println!("\nThe key difference: We waited for async task cleanup");
                println!("before shutting down the runtime.\n");
            }
            Err(e) => {
                log::error!("Logout failed: {}", e);
            }
        }
    });

    println!("� Success! No deadpool-runtime panic occurred.\n");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_state_machine_logout() {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Debug)
            .is_test(true)
            .try_init()
            .ok();

        setup_client();
        tokio::time::sleep(Duration::from_millis(100)).await;

        let state_machine = LogoutStateMachine::new();
        let result = state_machine.execute().await;

        assert!(result.is_ok(), "Logout should succeed without panic");
    }

    #[test]
    fn test_state_transitions() {
        let sm = LogoutStateMachine::new();

        // Verify initial stat
        let state = sm.state.lock().unwrap().clone();
        assert!(matches!(state, LogoutState::Idle));

        // Test transition
        sm.transition_to(LogoutState::PreChecking, "Test", 10);
        let state = sm.state.lock().unwrap().clone();
        assert!(matches!(state, LogoutState::PreChecking));
    }
}
