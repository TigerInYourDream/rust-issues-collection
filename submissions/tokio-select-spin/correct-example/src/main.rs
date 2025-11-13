//! Fixes the busy-loop by giving the idle branch a real wait period.
//!
//! Instead of awaiting `sleep(Duration::ZERO)`, we reuse a `tokio::time::Interval`
//! so the idle path yields control back to the scheduler and does not hog a core.

use env_logger::Env;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::{Instant, MissedTickBehavior};

const SPIN_LIMIT: usize = 150_000;
const PRODUCER_BURSTS: usize = 4;
const BURST_SIZE: usize = 25;

#[derive(Debug)]
struct ConsumerStats {
    processed: usize,
    idle_ticks: usize,
    duration_ms: u128,
}

async fn run_cooperative_consumer() -> ConsumerStats {
    let (tx, mut rx) = mpsc::unbounded_channel::<usize>();

    let consumer = tokio::spawn(async move {
        let mut processed = 0usize;
        let mut idle_ticks = 0usize;
        let start = Instant::now();
        let mut idle_interval = tokio::time::interval(Duration::from_millis(5));
        // Do not try to "catch up" if the consumer is busy; just wait for the next tick.
        idle_interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                biased;
                msg = rx.recv() => {
                    match msg {
                        Some(value) => {
                            processed += 1;
                            if processed.is_multiple_of(50) {
                                log::debug!("Processed {value}, total {processed}");
                            }
                        }
                        None => break,
                    }
                }
                _ = idle_interval.tick() => {
                    idle_ticks += 1;
                    if idle_ticks >= SPIN_LIMIT {
                        log::debug!("Reached idle tick guard ({SPIN_LIMIT})");
                        break;
                    }
                }
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
            tokio::time::sleep(Duration::from_millis(3)).await;
        }

        // Keep the channel open but idle for observation purposes.
        tokio::time::sleep(Duration::from_millis(30)).await;
        drop(tx);
    });

    let stats = consumer.await.expect("consumer task must finish");
    producer.await.expect("producer task must finish");

    stats
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let stats = run_cooperative_consumer().await;
    log::info!(
        "Processed {} messages in {} ms, idle ticks: {}",
        stats.processed,
        stats.duration_ms,
        stats.idle_ticks
    );

    if stats.idle_ticks < SPIN_LIMIT {
        println!(
            "✅ Idle branch cooperates with the scheduler ({} idle ticks < limit {}).",
            stats.idle_ticks, SPIN_LIMIT
        );
    } else {
        println!(
            "Unexpectedly hit idle guard ({} >= {}). Revisit the reproduction.",
            stats.idle_ticks, SPIN_LIMIT
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn prevents_busy_loop() {
        let stats = run_cooperative_consumer().await;
        assert!(
            stats.idle_ticks < SPIN_LIMIT,
            "Expected the interval throttle to avoid spinning, got {} idle ticks",
            stats.idle_ticks
        );
    }
}
