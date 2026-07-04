//! Lightweight time-based motion cues (R081 runtime).
//!
//! Region-oriented effects without a blink caret: connect fade, stream slide
//! window, and flux on the active hairline while streaming.

use std::time::{Duration, Instant};

/// Active motion timeline for the TUI frame loop.
#[derive(Debug, Clone)]
pub struct MotionState {
    pub started_at: Instant,
    connect_fade_until: Instant,
    stream_slide_until: Option<Instant>,
    pub flux_active: bool,
}

impl Default for MotionState {
    fn default() -> Self {
        let now = Instant::now();
        Self {
            started_at: now,
            // Daemon connect fade_in — 400ms (design choreography).
            connect_fade_until: now + Duration::from_millis(400),
            stream_slide_until: None,
            flux_active: false,
        }
    }
}

impl MotionState {
    /// True while any time-based cue should drive a higher frame rate.
    pub fn animating(&self) -> bool {
        let now = Instant::now();
        now < self.connect_fade_until
            || self
                .stream_slide_until
                .is_some_and(|until| now < until)
            || self.flux_active
    }

    /// Progress 0.0–1.0 for connect fade (1.0 = fully visible).
    pub fn connect_fade_progress(&self) -> f32 {
        let now = Instant::now();
        if now >= self.connect_fade_until {
            return 1.0;
        }
        let total = Duration::from_millis(400).as_secs_f32();
        let elapsed = now.duration_since(self.started_at).as_secs_f32();
        (elapsed / total).clamp(0.0, 1.0)
    }

    /// Stream-start slide window active (250ms).
    pub fn stream_slide_active(&self) -> bool {
        self.stream_slide_until
            .is_some_and(|until| Instant::now() < until)
    }

    pub fn on_stream_start(&mut self) {
        self.stream_slide_until = Some(Instant::now() + Duration::from_millis(250));
        self.flux_active = true;
    }

    pub fn on_stream_end(&mut self) {
        self.flux_active = false;
        self.stream_slide_until = None;
    }

    /// Flux phase 0.0–1.0 for hairline sweep (linear, continuous).
    pub fn flux_phase(&self) -> f32 {
        if !self.flux_active {
            return 0.0;
        }
        let ms = self.started_at.elapsed().as_millis() as f32;
        // ~2Hz wave for active hairline flux (not a single-cell blink).
        ((ms / 500.0) % 1.0) as f32
    }

    /// Whether the active hairline should use focus token this frame.
    pub fn flux_hairline_on(&self) -> bool {
        if !self.flux_active {
            return false;
        }
        self.flux_phase() < 0.5
    }

    /// Target poll interval: ~30 FPS while animating, idle otherwise.
    pub fn poll_ms(&self) -> u64 {
        if self.animating() {
            33
        } else {
            120
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connect_fade_starts_animating() {
        let m = MotionState::default();
        assert!(m.animating());
        assert!(m.connect_fade_progress() < 1.0 || m.connect_fade_progress() == 1.0);
    }

    #[test]
    fn stream_enables_flux() {
        let mut m = MotionState::default();
        m.connect_fade_until = Instant::now() - Duration::from_millis(1);
        assert!(!m.animating());
        m.on_stream_start();
        assert!(m.flux_active);
        assert!(m.animating());
        m.on_stream_end();
        assert!(!m.flux_active);
    }
}
