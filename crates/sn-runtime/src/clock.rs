use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// A monotonic clock used by the runtime scheduler.
///
/// Values are durations since the clock's origin; wall-clock time is deliberately
/// kept separate and remains available through QuickJS Date APIs.
pub trait RuntimeClock: Send + Sync + 'static {
    fn now(&self) -> Duration;
}

/// Production clock backed by Instant.
#[derive(Debug)]
pub struct MonotonicClock {
    origin: Instant,
}

impl Default for MonotonicClock {
    fn default() -> Self {
        Self {
            origin: Instant::now(),
        }
    }
}

impl RuntimeClock for MonotonicClock {
    fn now(&self) -> Duration {
        self.origin.elapsed()
    }
}

/// Deterministic clock for scheduler tests and controlled host simulations.
#[derive(Debug, Default)]
pub struct ManualClock {
    nanos: AtomicU64,
}

impl ManualClock {
    pub fn new(initial: Duration) -> Self {
        Self {
            nanos: AtomicU64::new(duration_to_nanos(initial)),
        }
    }

    /// Advances the clock to at least `value`; time never moves backwards.
    pub fn set(&self, value: Duration) {
        let target = duration_to_nanos(value);
        let _ = self
            .nanos
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
                Some(current.max(target))
            });
    }

    pub fn advance(&self, delta: Duration) {
        let increment = duration_to_nanos(delta);
        let _ = self
            .nanos
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
                Some(current.saturating_add(increment))
            });
    }
}

impl RuntimeClock for ManualClock {
    fn now(&self) -> Duration {
        Duration::from_nanos(self.nanos.load(Ordering::SeqCst))
    }
}

fn duration_to_nanos(value: Duration) -> u64 {
    value.as_nanos().min(u64::MAX as u128) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_clock_advances_and_saturates() {
        let clock = ManualClock::new(Duration::from_millis(5));
        clock.advance(Duration::from_millis(7));
        assert_eq!(clock.now(), Duration::from_millis(12));

        clock.set(Duration::from_millis(3));
        assert_eq!(clock.now(), Duration::from_millis(12));

        clock.set(Duration::MAX);
        clock.advance(Duration::from_secs(1));
        assert_eq!(clock.now(), Duration::from_nanos(u64::MAX));
    }
}
