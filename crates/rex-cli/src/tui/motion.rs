//! Time-based motion cues (R081 choreography).
//!
//! Region-oriented effects without a blink caret: connect fade, stream slide,
//! flux hairline, timeline coalesce, approval open/close, error shift.

use std::time::{Duration, Instant};

/// Active motion timeline for the TUI frame loop.
#[derive(Debug, Clone)]
pub struct MotionState {
    pub started_at: Instant,
    connect_fade_until: Instant,
    stream_slide_until: Option<Instant>,
    timeline_coalesce_until: Option<Instant>,
    approval_open_until: Option<Instant>,
    approval_close_until: Option<Instant>,
    error_shift_until: Option<Instant>,
    pub flux_active: bool,
    pub approval_visible: bool,
}

impl Default for MotionState {
    fn default() -> Self {
        let now = Instant::now();
        Self {
            started_at: now,
            connect_fade_until: now + Duration::from_millis(400),
            stream_slide_until: None,
            timeline_coalesce_until: None,
            approval_open_until: None,
            approval_close_until: None,
            error_shift_until: None,
            flux_active: false,
            approval_visible: false,
        }
    }
}

impl MotionState {
    fn until_active(until: Option<Instant>) -> bool {
        until.is_some_and(|t| Instant::now() < t)
    }

    /// True while any time-based cue should drive a higher frame rate.
    pub fn animating(&self) -> bool {
        let now = Instant::now();
        now < self.connect_fade_until
            || Self::until_active(self.stream_slide_until)
            || Self::until_active(self.timeline_coalesce_until)
            || Self::until_active(self.approval_open_until)
            || Self::until_active(self.approval_close_until)
            || Self::until_active(self.error_shift_until)
            || self.flux_active
    }

    pub fn connect_fade_progress(&self) -> f32 {
        let now = Instant::now();
        if now >= self.connect_fade_until {
            return 1.0;
        }
        let total = Duration::from_millis(400).as_secs_f32();
        let elapsed = now.duration_since(self.started_at).as_secs_f32();
        (elapsed / total).clamp(0.0, 1.0)
    }

    pub fn stream_slide_active(&self) -> bool {
        Self::until_active(self.stream_slide_until)
    }

    pub fn timeline_coalesce_active(&self) -> bool {
        Self::until_active(self.timeline_coalesce_until)
    }

    pub fn approval_opening(&self) -> bool {
        Self::until_active(self.approval_open_until)
    }

    pub fn approval_closing(&self) -> bool {
        Self::until_active(self.approval_close_until)
    }

    pub fn error_shift_active(&self) -> bool {
        Self::until_active(self.error_shift_until)
    }

    pub fn on_stream_start(&mut self) {
        self.stream_slide_until = Some(Instant::now() + Duration::from_millis(250));
        self.flux_active = true;
    }

    pub fn on_stream_end(&mut self) {
        self.flux_active = false;
        self.stream_slide_until = None;
    }

    /// Timeline task add — coalesce 300ms.
    pub fn on_timeline_add(&mut self) {
        self.timeline_coalesce_until = Some(Instant::now() + Duration::from_millis(300));
    }

    /// Approval open — dissolve/slide 350ms.
    pub fn on_approval_open(&mut self) {
        self.approval_visible = true;
        self.approval_open_until = Some(Instant::now() + Duration::from_millis(350));
        self.approval_close_until = None;
    }

    /// Approval close — 250ms.
    pub fn on_approval_close(&mut self) {
        self.approval_visible = false;
        self.approval_close_until = Some(Instant::now() + Duration::from_millis(250));
        self.approval_open_until = None;
    }

    /// Error — hsl shift toward error 300ms.
    pub fn on_error(&mut self) {
        self.error_shift_until = Some(Instant::now() + Duration::from_millis(300));
        self.on_stream_end();
    }

    pub fn flux_phase(&self) -> f32 {
        if !self.flux_active {
            return 0.0;
        }
        let ms = self.started_at.elapsed().as_millis() as f32;
        (ms / 500.0) % 1.0
    }

    pub fn flux_hairline_on(&self) -> bool {
        if !self.flux_active {
            return false;
        }
        self.flux_phase() < 0.5
    }

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

    #[test]
    fn approval_and_timeline_cues_animate() {
        let mut m = MotionState::default();
        m.connect_fade_until = Instant::now() - Duration::from_millis(1);
        m.on_timeline_add();
        assert!(m.timeline_coalesce_active());
        m.on_approval_open();
        assert!(m.approval_opening());
        assert!(m.approval_visible);
        m.on_approval_close();
        assert!(!m.approval_visible);
        assert!(m.approval_closing());
        m.on_error();
        assert!(m.error_shift_active());
    }
}
