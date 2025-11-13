use anyhow::Result;
use tokio::signal::unix::{signal, SignalKind};
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

#[tokio::main]
async fn main() -> Result<()> {
    println!("Hot-reload daemon (broken) - repeatedly registers signal handlers.");

    let mut leaked_handlers = Vec::new();

    for iteration in 0_usize.. {
        match signal(SignalKind::hangup()) {
            Ok(mut hup_stream) => {
                let handle = tokio::spawn(async move {
                    // Never awaited: the Signal handle lives forever causing fd leaks.
                    let _ = hup_stream.recv().await;
                    println!("[leaked task] SIGHUP observed.");
                });
                leaked_handlers.push(handle);

                println!(
                    "[broken] iteration={iteration:>4} | open_fds={}",
                    open_fd_count()
                );

                unsafe {
                    libc::raise(libc::SIGHUP);
                }
            }
            Err(err) => {
                eprintln!("[broken] fail iteration={iteration:>4}: cannot register signal: {err}");
                break;
            }
        }

        sleep(Duration::from_millis(25)).await;
    }

    println!("Keep the process alive to inspect /proc/self/fd or run lsof. Use Ctrl+C to exit.");
    std::future::pending::<()>().await;
    Ok(())
}
