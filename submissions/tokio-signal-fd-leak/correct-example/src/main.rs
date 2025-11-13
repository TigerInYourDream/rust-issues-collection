use anyhow::Result;
use once_cell::sync::OnceCell;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tokio::time::{sleep, Duration};

#[cfg(target_os = "linux")]
fn open_fd_count() -> usize {
    std::fs::read_dir("/proc/self/fd")
        .map(|dir| dir.count())
        .unwrap_or(0)
}

#[cfg(target_os = "macos")]
fn open_fd_count() -> usize {
    std::fs::read_dir("/dev/fd")
        .map(|dir| dir.count())
        .unwrap_or(0)
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn open_fd_count() -> usize {
    0
}

struct SignalHub {
    sender: broadcast::Sender<()>,
    _forwarder: JoinHandle<()>,
}

impl SignalHub {
    fn global() -> &'static Self {
        static HUB: OnceCell<SignalHub> = OnceCell::new();

        HUB.get_or_init(|| {
            let (tx, _) = broadcast::channel(32);
            let mut stream =
                signal(SignalKind::hangup()).expect("initialize single SIGHUP listener");
            let tx_clone = tx.clone();

            let forwarder = tokio::spawn(async move {
                while stream.recv().await.is_some() {
                    // Fan out a single signal to every subscriber.
                    let _ = tx_clone.send(());
                }
            });

            SignalHub {
                sender: tx,
                _forwarder: forwarder,
            }
        })
    }

    fn subscribe(&self) -> broadcast::Receiver<()> {
        self.sender.subscribe()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("Hot-reload daemon (fixed) - single signal stream reused.");
    let hub = SignalHub::global();

    for iteration in 0_usize..256 {
        let mut rx = hub.subscribe();

        tokio::spawn(async move {
            let _ = rx.recv().await;
            println!("[graceful task] reload triggered.");
        });

        println!(
            "[fixed ] iteration={iteration:>4} | open_fds={}",
            open_fd_count()
        );

        unsafe {
            libc::raise(libc::SIGHUP);
        }

        sleep(Duration::from_millis(25)).await;
    }

    Ok(())
}
