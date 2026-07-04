//! Tiered frame budget scheduler (Idle / Ambient / Active / Cinematic).

use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameTier {
    Idle,
    Ambient,
    Active,
    Cinematic,
}

#[derive(Debug)]
pub struct FrameBudget {
    tier: FrameTier,
    work_in_flight: bool,
    cinematic_until: Option<Instant>,
    last_activity: Instant,
    decorative_paused: bool,
    forced_ambient: bool,
}

impl Default for FrameBudget {
    fn default() -> Self {
        Self {
            tier: FrameTier::Idle,
            work_in_flight: false,
            cinematic_until: None,
            last_activity: Instant::now(),
            decorative_paused: false,
            forced_ambient: false,
        }
    }
}

impl FrameBudget {
    pub fn tier(&self) -> FrameTier {
        if self.decorative_paused && self.tier != FrameTier::Idle {
            return FrameTier::Ambient;
        }
        self.tier
    }

    pub fn poll_ms(&self) -> u64 {
        match self.tier() {
            FrameTier::Idle => 500,
            FrameTier::Ambient => 67,
            FrameTier::Active => 33,
            FrameTier::Cinematic => 17,
        }
    }

    pub fn on_input(&mut self) {
        self.last_activity = Instant::now();
        self.tier = FrameTier::Active;
    }

    pub fn on_work_start(&mut self) {
        self.work_in_flight = true;
        if self.tier == FrameTier::Idle {
            self.tier = FrameTier::Ambient;
        }
    }

    pub fn on_work_end(&mut self) {
        self.work_in_flight = false;
    }

    pub fn trigger_cinematic(&mut self, cap: Duration) {
        self.tier = FrameTier::Cinematic;
        self.cinematic_until = Some(Instant::now() + cap);
    }

    pub fn pause_decorative(&mut self) {
        self.decorative_paused = true;
    }

    pub fn resume_decorative(&mut self) {
        self.decorative_paused = false;
    }

    pub fn force_ambient_for_step(&mut self) {
        self.forced_ambient = true;
        self.tier = FrameTier::Ambient;
    }

    pub fn tick(&mut self) {
        if let Some(until) = self.cinematic_until {
            if Instant::now() >= until {
                self.cinematic_until = None;
                self.tier = if self.work_in_flight {
                    FrameTier::Ambient
                } else {
                    FrameTier::Active
                };
            }
        }
        if self.last_activity.elapsed() > Duration::from_secs(2) && !self.work_in_flight {
            if self.tier == FrameTier::Active {
                self.tier = FrameTier::Ambient;
            }
            if self.last_activity.elapsed() > Duration::from_secs(5) {
                self.tier = FrameTier::Idle;
            }
        }
        if !self.work_in_flight && self.tier == FrameTier::Ambient && self.last_activity.elapsed() > Duration::from_secs(2) {
            self.tier = FrameTier::Idle;
        }
        if self.forced_ambient {
            self.forced_ambient = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_poll_is_slow() {
        let b = FrameBudget::default();
        assert_eq!(b.poll_ms(), 500);
    }

    #[test]
    fn input_escalates_to_active() {
        let mut b = FrameBudget::default();
        b.on_input();
        assert_eq!(b.tier(), FrameTier::Active);
        assert_eq!(b.poll_ms(), 33);
    }
}
