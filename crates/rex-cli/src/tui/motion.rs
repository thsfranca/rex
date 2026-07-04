//! Region motion post-process (R081 + R090–R096 compositor).
//!
//! Widgets render first; the compositor effect graph mutates the ratatui buffer by region.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use super::compositor::{Compositor, Regions};
use super::theme::Theme;

/// Active motion timeline for the TUI frame loop.
#[derive(Debug)]
pub struct MotionState {
    inner: Compositor,
    pub approval_visible: bool,
    pub viewport: Rect,
    pub transcript: Rect,
    pub transcript_hairline: Rect,
    pub timeline: Rect,
    pub composer_hairline: Rect,
    pub composer: Rect,
    pub header: Rect,
}

impl Default for MotionState {
    fn default() -> Self {
        let mut s = Self {
            inner: Compositor::default(),
            approval_visible: false,
            viewport: Rect::default(),
            transcript: Rect::default(),
            transcript_hairline: Rect::default(),
            timeline: Rect::default(),
            composer_hairline: Rect::default(),
            composer: Rect::default(),
            header: Rect::default(),
        };
        s.inner.on_connect();
        s
    }
}

impl MotionState {
    pub fn animating(&self) -> bool {
        self.inner.animating()
    }

    pub fn wants_paint(&self) -> bool {
        self.inner.wants_paint()
    }

    pub fn poll_ms(&self) -> u64 {
        self.inner.poll_ms()
    }

    pub fn sync_output_enabled(&self, default: bool) -> bool {
        self.inner.sync_output_enabled(default)
    }

    pub fn set_reflow_paused(&mut self, paused: bool) {
        self.inner.set_reflow_paused(paused);
    }

    pub fn on_input(&mut self) {
        self.inner.on_input();
    }

    pub fn on_composer_input(&mut self) {
        self.inner.on_input();
        self.inner.graph.on_composer_input();
    }

    pub fn on_stream_start(&mut self) {
        self.inner.on_stream_start();
    }

    pub fn on_stream_end(&mut self) {
        self.inner.on_stream_end();
    }

    pub fn on_history_fetch_start(&mut self) {
        self.inner.on_history_fetch_start();
    }

    pub fn on_history_fetch_end(&mut self) {
        self.inner.on_history_fetch_end();
    }

    pub fn on_timeline_add(&mut self) {
        self.inner.on_timeline_add();
    }

    pub fn on_approval_open(&mut self) {
        self.approval_visible = true;
        self.inner.on_approval_open();
    }

    pub fn on_approval_close(&mut self) {
        self.approval_visible = false;
        self.inner.on_approval_close();
    }

    pub fn on_error(&mut self) {
        self.inner.on_error();
    }

    pub fn step_probe_clock(&mut self, ms: u64) {
        self.inner.step_probe_clock(ms);
    }

    pub fn expanded_timeline(&self) -> Option<usize> {
        self.inner.expanded_timeline
    }

    pub fn set_expanded_timeline(&mut self, idx: Option<usize>) {
        self.inner.expanded_timeline = idx;
    }

    pub fn diff_scrub_index(&self) -> usize {
        self.inner.diff_scrub_index
    }

    pub fn diff_scrub_max(&self) -> usize {
        self.inner.diff_scrub_max
    }

    pub fn set_diff_scrub_bounds(&mut self, max: usize) {
        self.inner.diff_scrub_max = max;
        if self.inner.diff_scrub_index > max {
            self.inner.diff_scrub_index = 0;
        }
    }

    pub fn diff_scrub_left(&mut self) {
        if self.inner.diff_scrub_index > 0 {
            self.inner.diff_scrub_index -= 1;
        }
    }

    pub fn diff_scrub_right(&mut self) {
        if self.inner.diff_scrub_index < self.inner.diff_scrub_max {
            self.inner.diff_scrub_index += 1;
        }
    }

    pub fn scroll_velocity(&self) -> f32 {
        self.inner.scroll_velocity
    }

    pub fn apply_scroll_momentum(&mut self, delta: i16) {
        self.inner.scroll_velocity = self.inner.scroll_velocity * 0.7 + delta as f32 * 0.3;
    }

    pub fn decay_scroll_momentum(&mut self) -> i16 {
        let v = self.inner.scroll_velocity;
        if v.abs() < 0.5 {
            self.inner.scroll_velocity = 0.0;
            return 0;
        }
        self.inner.scroll_velocity *= 0.85;
        v.round() as i16
    }

    pub fn process(&mut self, buf: &mut Buffer, theme: &Theme) {
        let regions = Regions {
            viewport: self.viewport,
            transcript: self.transcript,
            transcript_hairline: self.transcript_hairline,
            timeline: self.timeline,
            composer_hairline: self.composer_hairline,
            header: self.header,
            composer: self.composer,
        };
        self.approval_visible = self.inner.graph.approval_visible;
        self.inner.process(buf, &regions, theme);
    }

    pub fn damage_rects(&self, buf: &Buffer) -> Vec<Rect> {
        self.inner.dirty.damage_rects(buf)
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
        m.step_probe_clock(500);
        assert!(m.animating());
        m.on_stream_start();
        assert!(m.animating());
        m.on_stream_end();
    }

    #[test]
    fn braille_flux_mutates_hairline() {
        let mut m = MotionState::default();
        m.step_probe_clock(500);
        m.composer_hairline = Rect::new(0, 0, 10, 1);
        m.on_stream_start();
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 1));
        let theme = Theme::default_adaptive();
        m.step_probe_clock(100);
        m.process(&mut buf, &theme);
        let has_braille = (0..10).any(|x| {
            buf.cell((x, 0))
                .and_then(|c| c.symbol().chars().next())
                .map(|ch| ('\u{2800}'..='\u{28FF}').contains(&ch))
                .unwrap_or(false)
        });
        assert!(has_braille, "flux should paint braille on hairline region");
    }
}
