use anyhow::{Context, Result};
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;
use tokio::sync::oneshot;

const CLEANUP_TIMEOUT_MS: u64 = 500;

/// Inner Matrix client state that background tasks touch.
/// When the last Arc disappears we send a cleanup confirmation.
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

fn bootstrap_supervised_client() -> (Arc<ClientInner>, oneshot::Receiver<()>) {
    let (inner, drop_rx) = ClientInner::new(1337);
    spawn_background_tasks(&inner);
    (inner, drop_rx)
}

fn spawn_background_tasks(inner: &Arc<ClientInner>) {
    for task_id in 0..3 {
        let weak = Arc::downgrade(inner);
        tokio::spawn(async move {
            loop {
                match weak.upgrade() {
                    Some(state) => {
                        log::info!(
                            "task {task_id} borrowed Arc for client {} and released it before await",
                            state.id
                        );
                    }
                    None => {
                        log::info!(
                            "task {task_id} noticed client drop, exiting without holding Arc"
                        );
                        break;
                    }
                }

                tokio::time::sleep(Duration::from_millis(80)).await;
            }
        });
    }
}

async fn wait_for_cleanup(rx: oneshot::Receiver<()>) -> Result<()> {
    tokio::time::timeout(Duration::from_millis(CLEANUP_TIMEOUT_MS), rx)
        .await
        .context("cleanup wait timed out")?
        .context("drop sender dropped before signaling")
}

fn install_supervised_client() -> (oneshot::Receiver<()>, Weak<ClientInner>) {
    let (client, drop_rx) = bootstrap_supervised_client();
    log::info!("Strong count before logout: {}", Arc::strong_count(&client));
    let weak = Arc::downgrade(&client);
    drop(client);
    (drop_rx, weak)
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    log::info!("Running the supervised logout that uses Weak references in tasks");

    let (drop_rx, weak) = install_supervised_client();
    wait_for_cleanup(drop_rx).await?;

    log::info!(
        "Cleanup confirmed in time. Remaining strong references: {}",
        weak.strong_count()
    );

    tokio::time::sleep(Duration::from_millis(200)).await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "current_thread")]
    async fn logout_completes_when_tasks_use_weak() {
        let (drop_rx, weak) = install_supervised_client();
        wait_for_cleanup(drop_rx)
            .await
            .expect("cleanup should finish when tasks only hold Weak refs");
        assert_eq!(weak.strong_count(), 0);
    }
}
