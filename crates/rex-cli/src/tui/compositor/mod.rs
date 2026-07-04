//! Hybrid compositor: tiered frame budget, effect graph, animation clock.

mod braille;
mod clock;
mod dirty;
pub(crate) mod effects;
mod frame_budget;
mod hsl;
mod spring;

pub use clock::{AnimationClock, SteppedClock, SystemClock};
pub use dirty::DirtyTracker;
pub use effects::Regions;
pub use frame_budget::{FrameBudget, FrameTier};
pub use spring::SpringState;

use std::time::Duration;

use ratatui::buffer::Buffer;

use super::theme::Theme;

/// Effect graph + frame scheduler backing `MotionState`.
#[derive(Debug)]
pub struct Compositor {
    pub budget: FrameBudget,
    pub clock: SystemClock,
    pub stepped: SteppedClock,
    pub probe_mode: bool,
    pub graph: effects::EffectGraph,
    pub dirty: DirtyTracker,
    pub spring_modal: SpringState,
    pub spring_banner: SpringState,
    pub reflow_paused: bool,
    pub typing_burst_until: Option<std::time::Instant>,
    pub last_input: std::time::Instant,
    pub scroll_velocity: f32,
    pub expanded_timeline: Option<usize>,
    pub diff_scrub_index: usize,
    pub diff_scrub_max: usize,
    pub banner_active: bool,
}

impl Compositor {
    pub fn new(probe_mode: bool) -> Self {
        Self {
            budget: FrameBudget::default(),
            clock: SystemClock,
            stepped: SteppedClock::default(),
            probe_mode,
            graph: effects::EffectGraph::default(),
            dirty: DirtyTracker::default(),
            spring_modal: SpringState::modal_entrance(),
            spring_banner: SpringState::banner_drop(),
            reflow_paused: false,
            typing_burst_until: None,
            last_input: std::time::Instant::now(),
            scroll_velocity: 0.0,
            expanded_timeline: None,
            diff_scrub_index: 0,
            diff_scrub_max: 0,
            banner_active: false,
        }
    }

    pub fn now_ms(&self) -> u64 {
        if self.probe_mode {
            self.stepped.elapsed_ms()
        } else {
            self.clock.elapsed_ms()
        }
    }

    pub fn on_input(&mut self) {
        self.last_input = std::time::Instant::now();
        self.budget.on_input();
        self.typing_burst_until = Some(self.last_input + Duration::from_millis(500));
    }

    pub fn on_stream_start(&mut self) {
        self.graph.on_stream_start(self.now_ms());
        self.budget.on_work_start();
    }

    pub fn on_stream_end(&mut self) {
        self.graph.on_stream_end();
        self.budget.on_work_end();
    }

    pub fn on_history_fetch_start(&mut self) {
        self.graph.on_history_fetch_start(self.now_ms());
        self.budget.on_work_start();
    }

    pub fn on_history_fetch_end(&mut self) {
        self.graph.on_history_fetch_end();
        if !self.graph.flux_active {
            self.budget.on_work_end();
        }
    }

    pub fn on_timeline_add(&mut self) {
        self.graph.on_timeline_add(self.now_ms());
    }

    pub fn on_approval_open(&mut self) {
        self.graph.on_approval_open(self.now_ms());
        self.spring_modal.reset_entrance();
        self.budget.trigger_cinematic(Duration::from_millis(750));
    }

    pub fn on_approval_close(&mut self) {
        self.graph.on_approval_close(self.now_ms());
        self.budget.trigger_cinematic(Duration::from_millis(350));
    }

    pub fn on_error(&mut self) {
        self.graph.on_error(self.now_ms());
        self.banner_active = true;
        self.spring_banner.reset_entrance();
        self.budget.trigger_cinematic(Duration::from_millis(750));
    }

    pub fn on_connect(&mut self) {
        self.graph.on_connect(self.now_ms());
        self.budget.trigger_cinematic(Duration::from_millis(400));
    }

    pub fn set_reflow_paused(&mut self, paused: bool) {
        self.reflow_paused = paused;
        if paused {
            self.budget.pause_decorative();
        } else {
            self.budget.resume_decorative();
        }
    }

    pub fn animating(&self) -> bool {
        !self.reflow_paused && self.graph.animating(self.now_ms())
            || self.spring_modal.active()
            || self.spring_banner.active()
            || self.banner_active
    }

    pub fn wants_paint(&self) -> bool {
        if self.probe_mode && self.budget.tier() == FrameTier::Idle && !self.graph.has_pending_cues() {
            return false;
        }
        self.graph.wants_paint(self.now_ms())
            || self.spring_modal.active()
            || self.spring_banner.active()
    }

    pub fn poll_ms(&self) -> u64 {
        if self.probe_mode && self.budget.tier() == FrameTier::Idle {
            return 500;
        }
        self.budget.poll_ms()
    }

    pub fn sync_output_enabled(&self, default: bool) -> bool {
        if !default {
            return false;
        }
        if let Some(until) = self.typing_burst_until {
            if until > std::time::Instant::now() {
                return false;
            }
        }
        true
    }

    pub fn process(&mut self, buf: &mut Buffer, regions: &effects::Regions, theme: &Theme) {
        let now = self.now_ms();
        self.budget.tick();
        self.graph
            .process(buf, regions, theme, now, self.reflow_paused);
        if self.spring_modal.active() {
            self.spring_modal.step();
            effects::apply_spring_modal(buf, regions, self.spring_modal.offset_rows());
        }
        if self.banner_active && self.spring_banner.active() {
            self.spring_banner.step();
            effects::apply_banner_drop(buf, regions, self.spring_banner.offset_rows(), theme);
        } else if self.spring_banner.settled() && self.banner_active {
            self.banner_active = false;
        }
        self.dirty.snapshot(buf);
    }

    pub fn step_probe_clock(&mut self, ms: u64) {
        if self.probe_mode {
            self.stepped.step_ms(ms);
            self.budget.force_ambient_for_step();
        }
    }
}

impl Default for Compositor {
    fn default() -> Self {
        Self::new(crate::probe_context::is_tui_probe_fixture())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_escalates_on_stream() {
        let mut c = Compositor::new(false);
        c.on_stream_start();
        assert!(matches!(
            c.budget.tier(),
            FrameTier::Ambient | FrameTier::Active | FrameTier::Cinematic
        ));
    }

    #[test]
    fn probe_clock_steps_deterministically() {
        crate::tui::compositor::clock::reset_probe_clock_for_test();
        let mut c = Compositor::new(true);
        c.step_probe_clock(100);
        c.step_probe_clock(100);
        assert_eq!(c.now_ms(), 200);
    }
}
