//! Region motion post-process (R081 choreography).
//!
//! Widgets render first; effects mutate the ratatui buffer by region (same model as
//! tachyonfx). tachyonfx 0.25 depends on `ratatui-core` and is not type-compatible with
//! ratatui 0.29, so this module implements the design-table effects directly.
//! Idle with no active effects does not paint (Quiet ≥300ms for tuiwright).

use std::time::{Duration, Instant};

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CueKind {
    ConnectFade,
    StreamSlide,
    TimelineCoalesce,
    ApprovalOpen,
    ApprovalClose,
    ErrorShift,
}

#[derive(Debug, Clone)]
struct Cue {
    kind: CueKind,
    started: Instant,
    duration: Duration,
}

/// Active motion timeline for the TUI frame loop.
#[derive(Debug, Clone)]
pub struct MotionState {
    cues: Vec<Cue>,
    flux_active: bool,
    flux_started: Instant,
    /// Last hairline head cell painted (flux only paints when this advances).
    flux_head: Option<u16>,
    pub approval_visible: bool,
    /// Regions updated each draw before effects run.
    pub viewport: Rect,
    pub transcript: Rect,
    pub timeline: Rect,
    pub composer_hairline: Rect,
    pub header: Rect,
}

impl Default for MotionState {
    fn default() -> Self {
        let now = Instant::now();
        Self {
            cues: vec![Cue {
                kind: CueKind::ConnectFade,
                started: now,
                duration: Duration::from_millis(400),
            }],
            flux_active: false,
            flux_started: now,
            flux_head: None,
            approval_visible: false,
            viewport: Rect::default(),
            transcript: Rect::default(),
            timeline: Rect::default(),
            composer_hairline: Rect::default(),
            header: Rect::default(),
        }
    }
}

impl MotionState {
    fn push_cue(&mut self, kind: CueKind, ms: u64) {
        self.cues.retain(|c| c.kind != kind);
        self.cues.push(Cue {
            kind,
            started: Instant::now(),
            duration: Duration::from_millis(ms),
        });
    }

    fn progress(cue: &Cue) -> f32 {
        let elapsed = cue.started.elapsed().as_secs_f32();
        let total = cue.duration.as_secs_f32().max(0.001);
        (elapsed / total).clamp(0.0, 1.0)
    }

    fn ease_quad_out(t: f32) -> f32 {
        1.0 - (1.0 - t) * (1.0 - t)
    }

    fn ease_sine_out(t: f32) -> f32 {
        (t * std::f32::consts::FRAC_PI_2).sin()
    }

    fn ease_bounce_out(t: f32) -> f32 {
        // Lightweight bounce approximation.
        let n1 = 7.5625;
        let d1 = 2.75;
        if t < 1.0 / d1 {
            n1 * t * t
        } else if t < 2.0 / d1 {
            let t = t - 1.5 / d1;
            n1 * t * t + 0.75
        } else if t < 2.5 / d1 {
            let t = t - 2.25 / d1;
            n1 * t * t + 0.9375
        } else {
            let t = t - 2.625 / d1;
            n1 * t * t + 0.984375
        }
    }

    fn ease_quad_in_out(t: f32) -> f32 {
        if t < 0.5 {
            2.0 * t * t
        } else {
            1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
        }
    }

    /// True while any effect should drive a higher frame rate.
    pub fn animating(&self) -> bool {
        self.flux_active || self.cues.iter().any(|c| Self::progress(c) < 1.0)
    }

    /// Whether the next loop iteration should paint (region actually changes).
    /// Flux advances slowly so tuiwright Quiet (≥300ms) can succeed between frames.
    pub fn wants_paint(&self) -> bool {
        if self.cues.iter().any(|c| Self::progress(c) < 1.0) {
            return true;
        }
        if !self.flux_active {
            return false;
        }
        let width = self.composer_hairline.width.max(1);
        // One cell every 400ms → ≥300ms quiet windows between paints.
        let head = ((self.flux_started.elapsed().as_millis() / 400) as u16) % width;
        self.flux_head != Some(head)
    }

    pub fn on_stream_start(&mut self) {
        self.push_cue(CueKind::StreamSlide, 250);
        self.flux_active = true;
        self.flux_started = Instant::now();
        self.flux_head = None;
    }

    pub fn on_stream_end(&mut self) {
        self.flux_active = false;
        self.flux_head = None;
        self.cues.retain(|c| !matches!(c.kind, CueKind::StreamSlide));
    }

    pub fn on_timeline_add(&mut self) {
        self.push_cue(CueKind::TimelineCoalesce, 300);
    }

    pub fn on_approval_open(&mut self) {
        self.approval_visible = true;
        self.cues
            .retain(|c| !matches!(c.kind, CueKind::ApprovalClose));
        self.push_cue(CueKind::ApprovalOpen, 350);
    }

    pub fn on_approval_close(&mut self) {
        self.approval_visible = false;
        self.cues
            .retain(|c| !matches!(c.kind, CueKind::ApprovalOpen));
        self.push_cue(CueKind::ApprovalClose, 250);
    }

    pub fn on_error(&mut self) {
        self.on_stream_end();
        self.push_cue(CueKind::ErrorShift, 300);
    }

    /// Post-process the rendered buffer with active region effects.
    pub fn process(&mut self, buf: &mut Buffer) {
        self.cues.retain(|c| Self::progress(c) < 1.0);
        let cues: Vec<Cue> = self.cues.clone();
        for cue in &cues {
            let t = Self::progress(cue);
            match cue.kind {
                CueKind::ConnectFade => {
                    let a = Self::ease_quad_out(t);
                    // Fade from dim toward full luminance across viewport.
                    self.map_area(buf, self.viewport, |cell| {
                        if let Some(c) = cell.fg.into_rgb() {
                            let mix = |ch: u8| -> u8 {
                                let dim = (ch as f32 * 0.35) as u8;
                                (dim as f32 + (ch as f32 - dim as f32) * a) as u8
                            };
                            cell.set_fg(Color::Rgb(mix(c.0), mix(c.1), mix(c.2)));
                        }
                    });
                }
                CueKind::StreamSlide => {
                    let a = Self::ease_sine_out(t);
                    // Reveal transcript from bottom: dim upper rows early in the cue.
                    let area = self.transcript;
                    if area.height > 0 {
                        let reveal = (a * area.height as f32) as u16;
                        for y in area.y..area.y.saturating_add(area.height) {
                            let row = y.saturating_sub(area.y);
                            let visible = row + reveal >= area.height.saturating_sub(1);
                            if !visible {
                                for x in area.x..area.x.saturating_add(area.width) {
                                    if let Some(cell) = buf.cell_mut((x, y)) {
                                        cell.set_fg(Color::Reset);
                                        cell.set_char(' ');
                                    }
                                }
                            }
                        }
                    }
                }
                CueKind::TimelineCoalesce => {
                    let a = Self::ease_bounce_out(t);
                    let accent = (0x82u8, 0xA0u8, 0xFFu8);
                    self.map_area(buf, self.timeline, |cell| {
                        if let Some(c) = cell.fg.into_rgb() {
                            let mix = |ch: u8, ac: u8| -> u8 {
                                (ch as f32 + (ac as f32 - ch as f32) * (1.0 - a) * 0.6) as u8
                            };
                            cell.set_fg(Color::Rgb(
                                mix(c.0, accent.0),
                                mix(c.1, accent.1),
                                mix(c.2, accent.2),
                            ));
                        }
                    });
                }
                CueKind::ApprovalOpen => {
                    let a = Self::ease_quad_in_out(t);
                    // Dissolve backdrop: progressively clear non-modal cells.
                    let area = self.viewport;
                    for y in area.y..area.y.saturating_add(area.height) {
                        for x in area.x..area.x.saturating_add(area.width) {
                            let hash = ((x as u32).wrapping_mul(374761393)
                                ^ (y as u32).wrapping_mul(668265263))
                                % 1000;
                            if (hash as f32) / 1000.0 > a {
                                if let Some(cell) = buf.cell_mut((x, y)) {
                                    if let Some(c) = cell.fg.into_rgb() {
                                        cell.set_fg(Color::Rgb(
                                            c.0 / 2,
                                            c.1 / 2,
                                            c.2 / 2,
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
                CueKind::ApprovalClose => {
                    let a = Self::ease_quad_in_out(t);
                    // Slide out: blank bottom rows of viewport as cue progresses.
                    let area = self.viewport;
                    let blank = ((1.0 - a) * area.height as f32) as u16;
                    for y in area.y.saturating_add(area.height.saturating_sub(blank))
                        ..area.y.saturating_add(area.height)
                    {
                        for x in area.x..area.x.saturating_add(area.width) {
                            if let Some(cell) = buf.cell_mut((x, y)) {
                                cell.set_char(' ');
                            }
                        }
                    }
                }
                CueKind::ErrorShift => {
                    let a = t; // linear
                    self.map_area(buf, self.header, |cell| {
                        if let Some(c) = cell.fg.into_rgb() {
                            let mix = |ch: u8, err: u8| -> u8 {
                                (ch as f32 + (err as f32 - ch as f32) * a) as u8
                            };
                            cell.set_fg(Color::Rgb(
                                mix(c.0, 0xFF),
                                mix(c.1, 0x6B),
                                mix(c.2, 0x6B),
                            ));
                        }
                    });
                }
            }
        }

        if self.flux_active {
            // Sweep on composer hairline (multi-cell region, not one-cell blink).
            let area = self.composer_hairline;
            if area.width > 0 {
                let head = ((self.flux_started.elapsed().as_millis() / 400) as u16) % area.width;
                self.flux_head = Some(head);
                let accent = Color::Rgb(0x82, 0xA0, 0xFF);
                let dim = Color::Rgb(0x30, 0x31, 0x36);
                for i in 0..area.width {
                    let x = area.x.saturating_add(i);
                    let y = area.y;
                    if let Some(cell) = buf.cell_mut((x, y)) {
                        let dist = i.abs_diff(head);
                        let on = dist < 3;
                        cell.set_fg(if on { accent } else { dim });
                        if cell.symbol().trim().is_empty() {
                            cell.set_char('─');
                        }
                    }
                }
            }
        }
    }

    fn map_area(&self, buf: &mut Buffer, area: Rect, mut f: impl FnMut(&mut ratatui::buffer::Cell)) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        for y in area.y..area.y.saturating_add(area.height) {
            for x in area.x..area.x.saturating_add(area.width) {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    f(cell);
                }
            }
        }
    }

    pub fn poll_ms(&self) -> u64 {
        if self.animating() {
            33
        } else {
            120
        }
    }
}

trait IntoRgb {
    fn into_rgb(self) -> Option<(u8, u8, u8)>;
}

impl IntoRgb for Color {
    fn into_rgb(self) -> Option<(u8, u8, u8)> {
        match self {
            Color::Rgb(r, g, b) => Some((r, g, b)),
            Color::White | Color::Gray => Some((0xE2, 0xE2, 0xE2)),
            Color::DarkGray => Some((0x60, 0x61, 0x65)),
            Color::Black => Some((0x1A, 0x1B, 0x20)),
            Color::LightBlue | Color::Blue => Some((0x82, 0xA0, 0xFF)),
            Color::LightRed | Color::Red => Some((0xFF, 0x6B, 0x6B)),
            Color::LightGreen | Color::Green => Some((0x86, 0xE5, 0x9A)),
            Color::Reset => None,
            _ => Some((0xA0, 0xA0, 0xA5)),
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
        m.cues.clear();
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
        m.cues.clear();
        m.on_timeline_add();
        assert!(m.animating());
        m.on_approval_open();
        assert!(m.approval_visible);
        m.on_approval_close();
        assert!(!m.approval_visible);
        assert!(m.animating());
        m.on_error();
        assert!(m.animating());
    }

    #[test]
    fn flux_mutates_hairline_region() {
        let mut m = MotionState::default();
        m.cues.clear();
        m.composer_hairline = Rect::new(0, 0, 10, 1);
        m.on_stream_start();
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 1));
        for x in 0..10 {
            buf.cell_mut((x, 0)).unwrap().set_char('─');
        }
        m.process(&mut buf);
        let colors: Vec<_> = (0..10)
            .filter_map(|x| buf.cell((x, 0)).map(|c| c.fg))
            .collect();
        assert!(
            colors.iter().any(|c| matches!(c, Color::Rgb(0x82, 0xA0, 0xFF))),
            "flux should paint accent on hairline region"
        );
    }
}
