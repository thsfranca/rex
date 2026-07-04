//! Effect graph nodes and region post-process.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;

use super::braille;
use super::hsl;
use crate::tui::theme::Theme;

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
    started_ms: u64,
    duration_ms: u64,
}

#[derive(Debug, Clone, Default)]
pub struct Regions {
    pub viewport: Rect,
    pub transcript: Rect,
    pub transcript_hairline: Rect,
    pub timeline: Rect,
    pub composer_hairline: Rect,
    pub header: Rect,
    pub composer: Rect,
}

#[derive(Debug)]
pub struct EffectGraph {
    cues: Vec<Cue>,
    pub flux_active: bool,
    pub history_fetch_active: bool,
    flux_started_ms: u64,
    flux_head: Option<u16>,
    flux_on_transcript: bool,
    pub approval_visible: bool,
    composer_glow: f32,
}

impl Default for EffectGraph {
    fn default() -> Self {
        let now = 0u64;
        Self {
            cues: vec![Cue {
                kind: CueKind::ConnectFade,
                started_ms: now,
                duration_ms: 400,
            }],
            flux_active: false,
            history_fetch_active: false,
            flux_started_ms: now,
            flux_head: None,
            flux_on_transcript: false,
            approval_visible: false,
            composer_glow: 0.0,
        }
    }
}

impl EffectGraph {
    fn push_cue(&mut self, kind: CueKind, now_ms: u64, duration_ms: u64) {
        self.cues.retain(|c| c.kind != kind);
        self.cues.push(Cue {
            kind,
            started_ms: now_ms,
            duration_ms,
        });
    }

    fn progress(cue: &Cue, now_ms: u64) -> f32 {
        let elapsed = now_ms.saturating_sub(cue.started_ms) as f32;
        let total = cue.duration_ms.max(1) as f32;
        (elapsed / total).clamp(0.0, 1.0)
    }

    pub fn has_pending_cues(&self) -> bool {
        !self.cues.is_empty()
    }

    pub fn on_connect(&mut self, now_ms: u64) {
        self.push_cue(CueKind::ConnectFade, now_ms, 400);
    }

    pub fn on_stream_start(&mut self, now_ms: u64) {
        self.push_cue(CueKind::StreamSlide, now_ms, 250);
        self.flux_active = true;
        self.flux_on_transcript = false;
        self.flux_started_ms = now_ms;
        self.flux_head = None;
    }

    pub fn on_stream_end(&mut self) {
        self.flux_active = false;
        if !self.history_fetch_active {
            self.flux_head = None;
        }
        self.cues.retain(|c| !matches!(c.kind, CueKind::StreamSlide));
    }

    pub fn on_history_fetch_start(&mut self, now_ms: u64) {
        self.history_fetch_active = true;
        self.flux_on_transcript = true;
        self.flux_started_ms = now_ms;
        self.flux_head = None;
    }

    pub fn on_history_fetch_end(&mut self) {
        self.history_fetch_active = false;
        if !self.flux_active {
            self.flux_on_transcript = false;
            self.flux_head = None;
        }
    }

    pub fn on_timeline_add(&mut self, now_ms: u64) {
        self.push_cue(CueKind::TimelineCoalesce, now_ms, 300);
    }

    pub fn on_approval_open(&mut self, now_ms: u64) {
        self.approval_visible = true;
        self.cues.retain(|c| !matches!(c.kind, CueKind::ApprovalClose));
        self.push_cue(CueKind::ApprovalOpen, now_ms, 350);
    }

    pub fn on_approval_close(&mut self, now_ms: u64) {
        self.approval_visible = false;
        self.cues.retain(|c| !matches!(c.kind, CueKind::ApprovalOpen));
        self.push_cue(CueKind::ApprovalClose, now_ms, 250);
    }

    pub fn on_error(&mut self, now_ms: u64) {
        self.on_stream_end();
        self.push_cue(CueKind::ErrorShift, now_ms, 300);
    }

    pub fn on_composer_input(&mut self) {
        self.composer_glow = 1.0;
    }

    pub fn tick_composer_glow(&mut self) {
        self.composer_glow = (self.composer_glow - 0.08).max(0.0);
    }

    pub fn animating(&self, now_ms: u64) -> bool {
        self.flux_active
            || self.history_fetch_active
            || self.composer_glow > 0.01
            || self.cues.iter().any(|c| Self::progress(c, now_ms) < 1.0)
    }

    pub fn wants_paint(&self, now_ms: u64) -> bool {
        if self.cues.iter().any(|c| Self::progress(c, now_ms) < 1.0) {
            return true;
        }
        if !self.flux_active && !self.history_fetch_active {
            if self.composer_glow > 0.01 {
                return true;
            }
            return false;
        }
        let area = if self.flux_on_transcript {
            Rect::default()
        } else {
            Rect::default()
        };
        let _ = area;
        let interval_ms = 67u64;
        let head = ((now_ms.saturating_sub(self.flux_started_ms)) / interval_ms) as u16;
        self.flux_head != Some(head)
    }

    pub fn process(
        &mut self,
        buf: &mut Buffer,
        regions: &Regions,
        theme: &Theme,
        now_ms: u64,
        reflow_paused: bool,
    ) {
        self.cues
            .retain(|c| Self::progress(c, now_ms) < 1.0);
        let cues: Vec<Cue> = self.cues.clone();
        for cue in &cues {
            let t = Self::progress(cue, now_ms);
            match cue.kind {
                CueKind::ConnectFade => {
                    let a = ease_quad_out(t);
                    map_area(buf, regions.viewport, |cell| {
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
                    let a = ease_sine_out(t);
                    let area = regions.transcript;
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
                    let a = ease_bounce_out(t);
                    let accent = theme.text_accent.into_rgb().unwrap_or((0x82, 0xA0, 0xFF));
                    map_area(buf, regions.timeline, |cell| {
                        if let Some(c) = cell.fg.into_rgb() {
                            cell.set_fg(hsl::lerp_rgb(c, accent, (1.0 - a) * 0.6));
                        }
                    });
                }
                CueKind::ApprovalOpen => {
                    let a = ease_quad_in_out(t);
                    map_area(buf, regions.viewport, |cell| {
                        if let Some(c) = cell.fg.into_rgb() {
                            cell.set_fg(hsl::dim_luminosity(c, 1.0 - a * 0.55));
                        }
                        if let Some(c) = cell.bg.into_rgb() {
                            cell.set_bg(hsl::dim_luminosity(c, 1.0 - a * 0.45));
                        }
                    });
                }
                CueKind::ApprovalClose => {
                    let a = ease_quad_in_out(t);
                    let area = regions.viewport;
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
                    let err = theme.status_error.into_rgb().unwrap_or((0xFF, 0x6B, 0x6B));
                    map_area(buf, regions.header, |cell| {
                        if let Some(c) = cell.fg.into_rgb() {
                            cell.set_fg(hsl::lerp_rgb(c, err, t));
                        }
                    });
                }
            }
        }

        if !reflow_paused && (self.flux_active || self.history_fetch_active) {
            let area = if self.flux_on_transcript {
                regions.transcript_hairline
            } else {
                regions.composer_hairline
            };
            paint_braille_flux(buf, area, theme, now_ms, self.flux_started_ms, &mut self.flux_head);
        }

        if self.composer_glow > 0.01 && !regions.composer.is_empty() {
            apply_composer_glow(buf, regions.composer, self.composer_glow, theme);
            self.tick_composer_glow();
        }
    }
}

fn paint_braille_flux(
    buf: &mut Buffer,
    area: Rect,
    theme: &Theme,
    now_ms: u64,
    flux_started_ms: u64,
    flux_head: &mut Option<u16>,
) {
    if area.width == 0 {
        return;
    }
    let interval_ms = 67u64;
    let head = ((now_ms.saturating_sub(flux_started_ms)) / interval_ms) as u16 % area.width;
    *flux_head = Some(head);
    let accent = theme.text_accent;
    let dim = theme.hairline_default;
    for i in 0..area.width {
        let x = area.x.saturating_add(i);
        let y = area.y;
        if let Some(cell) = buf.cell_mut((x, y)) {
            let dist = i.abs_diff(head);
            let phase = ((now_ms / 16) as u8).wrapping_add(i as u8);
            let on = dist < 4;
            cell.set_fg(if on { accent } else { dim });
            cell.set_char(braille::flux_glyph(phase));
        }
    }
}

fn apply_composer_glow(buf: &mut Buffer, area: Rect, glow: f32, theme: &Theme) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let row = area.y.saturating_add(area.height.saturating_sub(1));
    let x = area.x.saturating_add(area.width.saturating_sub(1));
    if let Some(cell) = buf.cell_mut((x, row)) {
        let base = theme.surface_raised.into_rgb().unwrap_or((0x1A, 0x1B, 0x20));
        cell.set_bg(hsl::lerp_rgb(base, (0x82, 0xA0, 0xFF), glow * 0.25));
    }
}

pub fn apply_spring_modal(buf: &mut Buffer, regions: &Regions, offset_rows: i16) {
    if offset_rows == 0 {
        return;
    }
    let blur = match offset_rows.abs() {
        0..=1 => '▃',
        _ => '▆',
    };
    let y = regions.viewport.y.saturating_add((offset_rows.max(0)) as u16);
    for x in regions.viewport.x..regions.viewport.x.saturating_add(regions.viewport.width) {
        if let Some(cell) = buf.cell_mut((x, y.min(buf.area.height.saturating_sub(1)))) {
            cell.set_char(blur);
        }
    }
}

pub fn apply_banner_drop(buf: &mut Buffer, regions: &Regions, offset_rows: i16, theme: &Theme) {
    let y = regions.header.y.saturating_add((offset_rows.max(0)) as u16);
    if y >= buf.area.height {
        return;
    }
    for x in regions.header.x..regions.header.x.saturating_add(regions.header.width) {
        if let Some(cell) = buf.cell_mut((x, y)) {
            cell.set_bg(theme.status_error);
            if x == regions.header.x {
                cell.set_char('⚠');
            }
        }
    }
}

pub fn apply_diff_scrub_line(buf: &mut Buffer, area: Rect, added: bool, intensity: f32) {
    let color = if added {
        hsl::diff_scrub_green(intensity)
    } else {
        hsl::diff_scrub_red(intensity)
    };
    map_area(buf, area, |cell| {
        cell.set_bg(color);
    });
}

fn map_area(buf: &mut Buffer, area: Rect, mut f: impl FnMut(&mut ratatui::buffer::Cell)) {
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

fn ease_quad_out(t: f32) -> f32 {
    1.0 - (1.0 - t) * (1.0 - t)
}

fn ease_sine_out(t: f32) -> f32 {
    (t * std::f32::consts::FRAC_PI_2).sin()
}

fn ease_bounce_out(t: f32) -> f32 {
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
