//! Injectable animation clock for production and harness determinism.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

static PROBE_ELAPSED_MS: AtomicU64 = AtomicU64::new(0);

/// Wall-clock time source for production TUI runs.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl SystemClock {
    pub fn elapsed_ms(&self) -> u64 {
        static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
        let start = START.get_or_init(Instant::now);
        start.elapsed().as_millis() as u64
    }
}

/// Deterministic clock advanced only by explicit steps in probe harness mode.
#[derive(Debug, Clone, Default)]
pub struct SteppedClock;

impl SteppedClock {
    pub fn elapsed_ms(&self) -> u64 {
        PROBE_ELAPSED_MS.load(Ordering::Relaxed)
    }

    pub fn step_ms(&mut self, ms: u64) {
        PROBE_ELAPSED_MS.fetch_add(ms, Ordering::Relaxed);
    }
}

pub trait AnimationClock {
    fn elapsed_ms(&self) -> u64;
}

impl AnimationClock for SystemClock {
    fn elapsed_ms(&self) -> u64 {
        self.elapsed_ms()
    }
}

impl AnimationClock for SteppedClock {
    fn elapsed_ms(&self) -> u64 {
        self.elapsed_ms()
    }
}

/// Advance stepped animation clock when running in the tuiwright probe fixture.
pub fn advance_fixture_clock_ms(ms: u64) {
    if crate::probe_context::is_tui_probe_fixture() {
        PROBE_ELAPSED_MS.fetch_add(ms, Ordering::Relaxed);
    }
}

#[cfg(test)]
pub fn reset_probe_clock_for_test() {
    PROBE_ELAPSED_MS.store(0, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stepped_clock_accumulates() {
        reset_probe_clock_for_test();
        let mut c = SteppedClock;
        c.step_ms(16);
        c.step_ms(16);
        assert_eq!(c.elapsed_ms(), 32);
    }
}
