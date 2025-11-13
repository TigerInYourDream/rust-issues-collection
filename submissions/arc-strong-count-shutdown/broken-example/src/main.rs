use anyhow::{Context, Result};
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;
use tokio::sync::oneshot;

const CLEANUP_TIMEOUT_MS: u64 = 300;

/// Inner Matrix client state that background tasks need to access.
/// We simulate drop-time cleanup by sending through a oneshot channel.
struct ClientInner {
    id: u64,
    drop_tx: Mutex<Option<oneshot::Sender<()>>>,
}

impl ClientInner {
    fn new(id: u64) -> (Arc<Self>, oneshot::Receiver<()>) {
        let (tx, rx) = oneshot::channel::<()>();
        let inner = Arc::new(Self {
            id,
            drop_tx: Mutex::new(Some(tx)),
        });
        (inner, rx)
    }
}

impl Drop for ClientInner {
    fn drop(&mut self) {
        log::info!("ClientInner::drop executed for client {}", self.id);
        if let Some(tx) = self.drop_tx.lock().unwrap().take() {
            let _ = tx.send(());
        }
    }
}

fn bootstrap_leaky_client() -> (Arc<ClientInner>, oneshot::Receiver<()>) {
    let (inner, drop_rx) = ClientInner::new(42);
    spawn_background_tasks(inner.clone());
    (inner, drop_rx)
}

fn spawn_background_tasks(inner: Arc<ClientInner>) {
    for task_id in 0..3 {
        let task_inner = inner.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(80)).await;
                // Each task owns a strong Arc, preventing ClientInner::drop from running.
                log::info!(
                    "task {task_id} still owns Arc for client {} (leak by design)",
                    task_inner.id
                );
            }
        });
    }
}

async fn wait_for_cleanup(rx: oneshot::Receiver<()>) -> Result<()> {
    tokio::time::timeout(Duration::from_millis(CLEANUP_TIMEOUT_MS), rx)
        .await
        .context("logout stalled: client drop signal never arrived")?
        .context("drop sender dropped before sending signal")
}

fn install_leaky_client() -> (oneshot::Receiver<()>, Weak<ClientInner>) {
    let (client, drop_rx) = bootstrap_leaky_client();
    let weak = Arc::downgrade(&client);
    log::info!("Strong count before logout: {}", Arc::strong_count(&client));
    drop(client);
    (drop_rx, weak)
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    log::info!("Demonstrating how background tasks retaining Arc<Inner> block logout cleanup");

    let (drop_rx, weak) = install_leaky_client();

    match wait_for_cleanup(drop_rx).await {
        Ok(_) => {
            log::info!("Unexpected success: cleanup notification arrived");
        }
        Err(err) => {
            log::error!("Logout failed: {err:#}");
            log::error!(
                "Strong count after dropping last user handle: {}",
                weak.strong_count()
            );
            log::error!("Drop handler never fired because background tasks leaked the Arc");
        }
    }

    // Let logs from spawned tasks flush before runtime exits.
    tokio::time::sleep(Duration::from_millis(200)).await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "current_thread")]
    async fn logout_times_out_when_tasks_hold_arc() {
        let (drop_rx, weak) = install_leaky_client();
        let result = wait_for_cleanup(drop_rx).await;
        assert!(
            result.is_err(),
            "cleanup should time out when Arc is leaked"
        );
        assert!(
            weak.strong_count() > 0,
            "background tasks should still own the Arc"
        );
    }
}
