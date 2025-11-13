# Tokio Signal Handler File Descriptor Leak

## Problem Overview

A critical resource leak in Tokio-based services that repeatedly create signal handlers for hot-reload functionality. Each call to `signal()` registers a new signalfd/self-pipe with the kernel, but these file descriptors are never released, eventually causing "Too many open files" errors and complete signal handling failure.

## The Issue

When implementing hot-reload functionality triggered by SIGHUP signals, a naive approach of creating a new signal handler for each reload causes file descriptor exhaustion:

```rust
// BROKEN: Creates new FD on every iteration
loop {
    let mut hup = signal(SignalKind::hangup())?;  // New FD registered here!
    tokio::spawn(async move {
        let _ = hup.recv().await;
        trigger_reload().await;
    });
}
```

### Why This Fails

1. **Each `signal()` call registers a new file descriptor** with the kernel (signalfd on Linux, self-pipe on other Unix systems)
2. **Old signal handlers are never dropped** - they're moved into spawned tasks that wait indefinitely
3. **File descriptors accumulate** until hitting the process limit (`ulimit -n`)
4. **Once limit is reached**, no new signals can be registered, breaking hot-reload permanently

## Real-World Impact

This issue affects long-running services that need configuration hot-reload:
- Web servers and API gateways
- Proxy services
- Media streaming servers
- Background job processors
- Any daemon that uses SIGHUP for configuration reload

In production, services would run fine for days, then suddenly:
1. Monitoring alerts for high FD usage
2. Some instances stop responding to SIGHUP
3. Configuration updates fail silently
4. Eventually: complete signal handling failure

## The Solution

Create a single signal handler and distribute events via broadcast channel:

```rust
// CORRECT: Single handler with broadcast distribution
static SIGNAL_HANDLER: OnceCell<SignalHandler> = OnceCell::new();

struct SignalHandler {
    tx: broadcast::Sender<()>,
}

impl SignalHandler {
    fn new() -> Self {
        let (tx, _) = broadcast::channel(16);

        // Register signal handler ONCE
        tokio::spawn(async move {
            let mut signal = signal(SignalKind::hangup()).unwrap();
            loop {
                signal.recv().await;
                let _ = tx.send(());
            }
        });

        Self { tx }
    }

    fn subscribe(&self) -> broadcast::Receiver<()> {
        self.tx.subscribe()
    }
}
```

## Project Structure

```
tokio-signal-fd-leak/
|-- broken-example/      # Demonstrates the FD leak
|   |-- src/
|   |   `-- main.rs     # Naive implementation that leaks FDs
|   `-- Cargo.toml
|-- correct-example/     # Shows the proper solution
|   |-- src/
|   |   `-- main.rs     # Singleton handler with broadcast
|   `-- Cargo.toml
`-- README.md           # This file
```

## Running the Examples

### Reproducing the Problem

```bash
cd broken-example

# Lower FD limit to accelerate failure
ulimit -n 256

# Run and watch FD count grow
cargo run

# Output shows increasing FD count until:
# "cannot register signal: Os { code: 24, kind: Other, message: "Too many open files" }"
```

### Verifying the Solution

```bash
cd correct-example

# Even with low limit, runs indefinitely
ulimit -n 256
cargo run

# FD count remains stable no matter how many reloads
```

## Debugging Commands

Monitor file descriptor usage in real-time:

```bash
# Show open FDs for a process
lsof -p <PID> | wc -l

# Watch FD count over time
watch -n1 "ls /proc/<PID>/fd | wc -l"

# See actual file descriptors
ls -la /proc/<PID>/fd/
```

## Technical Details

### How Tokio Signal Handling Works

1. `tokio::signal::unix::signal()` uses `signal-hook-mio` internally
2. On Linux: Creates a signalfd for efficient signal delivery
3. On other Unix: Uses the self-pipe trick (signal handler writes to pipe)
4. Each call registers a NEW file descriptor with the kernel
5. FD is only released when the `Signal` struct is dropped

### Why the Leak Occurs

The broken pattern creates a perfect storm:
- New `Signal` created on each iteration
- Old `Signal` moved into spawned task
- Task holds `Signal` waiting for next signal (which never comes)
- `Signal` never dropped = FD never released
- Process eventually hits `EMFILE` (Error 24: Too many open files)

### The Broadcast Solution

The correct approach separates concerns:
- **Signal Reception**: One task owns the single `Signal` instance
- **Event Distribution**: Broadcast channel fans out to multiple consumers
- **Lifecycle Management**: Signal handler lives for entire process lifetime
- **Clean Shutdown**: Can explicitly stop the handler task when needed

## Common Mistakes

1. **Creating signal handlers in loops** - Always leads to FD leak
2. **Not reusing signal handlers** - Each handler costs an FD
3. **Forgetting to drop old handlers** - FDs persist until explicitly released
4. **Assuming GC will help** - Rust has no GC; resources need explicit management

## Production Considerations

### Monitoring

Add metrics for:
- Open file descriptor count
- Signal handler registration failures
- Time since last successful reload

### Graceful Shutdown

The correct example can be extended with shutdown support:

```rust
impl SignalHandler {
    async fn shutdown(self) {
        // Cancel the signal listening task
        // Close broadcast channel
        // Ensure clean process exit
    }
}
```

### Resource Limits

Check and adjust limits appropriately:

```bash
# Check current limits
ulimit -n

# Increase for production (in systemd unit or init script)
LimitNOFILE=65536
```

## Environment

- **Rust Version**: 1.75+ (async/await support)
- **Operating Systems**: Linux, macOS, FreeBSD, other Unix
- **Not supported**: Windows (Unix signals only)
- **Dependencies**:
  - tokio (with "signal" feature)
  - tokio-util (for broadcast channel)
  - anyhow (error handling)

## Quality Assurance

Both examples pass:
- `cargo check` - Compilation verification
- `cargo clippy` - Linting
- `cargo fmt` - Code formatting
- `cargo test` - Unit tests (correct example only)

## Key Takeaways

1. **Tokio signal handlers are expensive** - each one consumes an FD
2. **Create once, use many** - singleton pattern prevents resource leaks
3. **Broadcast for fan-out** - efficient distribution to multiple consumers
4. **Monitor resource usage** - FD leaks are silent until catastrophic
5. **Test with low limits** - reproduce issues quickly with `ulimit -n`

## References

- [Tokio Signal Documentation](https://docs.rs/tokio/latest/tokio/signal/index.html)
- [signal-hook Crate](https://docs.rs/signal-hook/latest/signal_hook/)
- [Linux signalfd(2) Man Page](https://man7.org/linux/man-pages/man2/signalfd.2.html)
- [Self-pipe Trick Explanation](https://cr.yp.to/docs/selfpipe.html)

## License

MIT
