use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Tracks in-flight work and client presence for idle lifecycle and shutdown.
pub struct ActivityTracker {
    active_work: AtomicU64,
    last_work_at: Mutex<Instant>,
    last_client_contact_at: Mutex<Instant>,
}

pub struct WorkGuard<'a> {
    tracker: &'a ActivityTracker,
}

impl Drop for WorkGuard<'_> {
    fn drop(&mut self) {
        let previous = self.tracker.active_work.fetch_sub(1, Ordering::AcqRel);
        if previous == 1 {
            *self
                .tracker
                .last_work_at
                .lock()
                .expect("activity last_work_at mutex should not be poisoned") = Instant::now();
        }
    }
}

impl ActivityTracker {
    pub fn new(started_at: Instant) -> Self {
        Self {
            active_work: AtomicU64::new(0),
            last_work_at: Mutex::new(started_at),
            last_client_contact_at: Mutex::new(started_at),
        }
    }

    pub fn track_work(&self) -> WorkGuard<'_> {
        self.active_work.fetch_add(1, Ordering::AcqRel);
        WorkGuard { tracker: self }
    }

    pub fn record_client_contact(&self) {
        *self
            .last_client_contact_at
            .lock()
            .expect("activity last_client_contact_at mutex should not be poisoned") =
            Instant::now();
    }

    pub fn is_busy(&self) -> bool {
        self.active_work.load(Ordering::Acquire) > 0
    }

    pub fn lifecycle_state(&self) -> &'static str {
        if self.is_busy() {
            "ready"
        } else {
            "idle"
        }
    }

    pub fn idle_seconds(&self) -> u64 {
        if self.is_busy() {
            return 0;
        }
        self.last_work_at
            .lock()
            .expect("activity last_work_at mutex should not be poisoned")
            .elapsed()
            .as_secs()
    }

    pub fn seconds_until_shutdown(&self, budget_secs: u64) -> u64 {
        if budget_secs == 0 {
            return 0;
        }
        let elapsed = self.last_activity_at().elapsed().as_secs();
        budget_secs.saturating_sub(elapsed)
    }

    pub fn should_shutdown(&self, budget_secs: u64) -> bool {
        if budget_secs == 0 || self.is_busy() {
            return false;
        }
        self.last_activity_at().elapsed() >= Duration::from_secs(budget_secs)
    }

    fn last_activity_at(&self) -> Instant {
        let work = *self
            .last_work_at
            .lock()
            .expect("activity last_work_at mutex should not be poisoned");
        let contact = *self
            .last_client_contact_at
            .lock()
            .expect("activity last_client_contact_at mutex should not be poisoned");
        work.max(contact)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration as StdDuration;

    #[test]
    fn lifecycle_flips_between_ready_and_idle() {
        let tracker = ActivityTracker::new(Instant::now());
        assert_eq!(tracker.lifecycle_state(), "idle");
        let guard = tracker.track_work();
        assert_eq!(tracker.lifecycle_state(), "ready");
        drop(guard);
        assert_eq!(tracker.lifecycle_state(), "idle");
    }

    #[test]
    fn client_contact_delays_shutdown() {
        let tracker = ActivityTracker::new(Instant::now() - StdDuration::from_secs(5));
        assert!(tracker.should_shutdown(3));
        tracker.record_client_contact();
        assert!(!tracker.should_shutdown(3));
    }

    #[test]
    fn active_work_prevents_shutdown() {
        let tracker = ActivityTracker::new(Instant::now() - StdDuration::from_secs(5));
        let _guard = tracker.track_work();
        assert!(!tracker.should_shutdown(1));
    }

    #[test]
    fn nested_work_guards_balance() {
        let tracker = ActivityTracker::new(Instant::now());
        let outer = tracker.track_work();
        let inner = tracker.track_work();
        drop(inner);
        assert!(tracker.is_busy());
        drop(outer);
        assert!(!tracker.is_busy());
    }

    #[test]
    fn idle_seconds_zero_while_busy() {
        let tracker = ActivityTracker::new(Instant::now() - StdDuration::from_secs(10));
        let _guard = tracker.track_work();
        assert_eq!(tracker.idle_seconds(), 0);
    }

    #[test]
    fn concurrent_work_tracking() {
        let tracker = Arc::new(ActivityTracker::new(Instant::now()));
        let handles: Vec<_> = (0..8)
            .map(|_| {
                let tracker = Arc::clone(&tracker);
                thread::spawn(move || {
                    let _guard = tracker.track_work();
                    thread::sleep(StdDuration::from_millis(10));
                })
            })
            .collect();
        for handle in handles {
            handle.join().expect("thread join");
        }
        assert!(!tracker.is_busy());
    }
}
