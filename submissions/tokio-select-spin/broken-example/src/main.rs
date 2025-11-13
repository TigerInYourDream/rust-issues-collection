//! Demonstrates how a `tokio::select!` loop can spin at 100% CPU when a biased
//! branch awaits an immediately-ready future (a stand-in for `sleep(Duration::ZERO)`).
//!
//! The consumer tries to avoid blocking on `recv()` by adding a "zero-delay"
//! branch. Because the branch completes instantly (and is prioritised via
//! `biased`), the loop never yields to Tokio and burns a full core whenever the
//! channel is temporarily empty.

use env_logger::Env;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::Instant;

const SPIN_LIMIT: usize = 150_000;
const PRODUCER_BURSTS: usize = 4;
const BURST_SIZE: usize = 25;

#[derive(Debug)]
struct ConsumerStats {
    processed: usize,
    idle_ticks: usize,
    duration_ms: u128,
}

async fn run_busy_consumer() -> ConsumerStats {
    let (tx, mut rx) = mpsc::unbounded_channel::<usize>();

    let consumer = tokio::spawn(async move {
        let mut processed = 0usize;
        let mut idle_ticks = 0usize;
        let start = Instant::now();

        loop {
            tokio::select! {
                biased;
                // This future completes immediately, mimicking `sleep(Duration::ZERO)`.
                _ = async {} => {
                    idle_ticks += 1;
                    if idle_ticks >= SPIN_LIMIT {
                        log::debug!("Hit spin limit ({SPIN_LIMIT})");
                        break;
                    }
                    // Nothing to await here: the loop immediately iterates again,
                    // producing a hot spin.
                }
                message = rx.recv() => {
                    match message {
                        Some(value) => {
                            processed += 1;
                            if processed.is_multiple_of(50) {
                                log::debug!("Processed {value}, total {processed}");
                            }
                        }
                        None => break,
                    }
                },
            }
        }

        ConsumerStats {
            processed,
            idle_ticks,
            duration_ms: start.elapsed().as_millis(),
        }
    });

    let producer = tokio::spawn(async move {
        for burst in 0..PRODUCER_BURSTS {
            for i in 0..BURST_SIZE {
                let payload = burst * BURST_SIZE + i;
                if tx.send(payload).is_err() {
                    log::debug!("Receiver dropped unexpectedly");
                    return;
                }
            }
            // Intentionally pause so the recipient sees an empty queue while the
            // sender still exists.
            tokio::time::sleep(Duration::from_millis(3)).await;
        }

        // Keep the channel alive but idle long enough for the busy loop to show.
        tokio::time::sleep(Duration::from_millis(30)).await;
        drop(tx);
    });

    let stats = consumer.await.expect("consumer task should not panic");
    producer.await.expect("producer task should not panic");

    stats
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let stats = run_busy_consumer().await;
    log::info!(
        "Processed {} messages in {} ms, idle ticks: {}",
        stats.processed,
        stats.duration_ms,
        stats.idle_ticks
    );

    if stats.idle_ticks >= SPIN_LIMIT {
        println!(
            "⚠️ Busy loop detected: hit the spin limit ({SPIN_LIMIT}). \
             The biased select! kept the zero-duration sleep branch hot."
        );
    } else {
        println!(
            "Idle ticks below threshold ({}), adjust the reproduction.",
            stats.idle_ticks
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn demonstrates_busy_loop() {
        let stats = run_busy_consumer().await;
        assert!(
            stats.idle_ticks >= SPIN_LIMIT,
            "Expected the biased zero-duration sleep to spin, got {} idle ticks",
            stats.idle_ticks
        );
    }
}
