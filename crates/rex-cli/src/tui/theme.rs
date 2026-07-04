//! Semantic style tokens for the terminal harness (`docs/TUI_DESIGN.md`).
//!
//! All TUI colors resolve through this map. Ad-hoc colors outside these tokens
//! fail design review.

use ratatui::style::{Color, Modifier, Style};

/// Palette used by the TUI. Colors enhance glyphs; monochrome remains usable.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    /// Transcript background (transparent / reset per design tokens).
    #[allow(dead_code)]
    pub surface_base: Color,
    pub surface_raised: Color,
    pub surface_overlay: Color,
    pub surface_dimmed: Color,
    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_tertiary: Color,
    pub text_accent: Color,
    pub hairline_default: Color,
    pub hairline_focus: Color,
    pub status_success: Color,
    pub status_warning: Color,
    pub status_error: Color,
    pub status_working: Color,
    pub status_idle: Color,
    /// When true, prefer weight/glyphs over color (NO_COLOR).
    pub monochrome: bool,
}

impl Theme {
    /// Build the adaptive palette: truecolor when available, else 16-color ANSI.
    /// Honors `NO_COLOR` (no-color.org).
    pub fn default_adaptive() -> Self {
        if no_color_set() {
            return Self::monochrome();
        }
        if supports_truecolor() {
            Self::truecolor()
        } else {
            Self::ansi16()
        }
    }

    fn monochrome() -> Self {
        Self {
            surface_base: Color::Reset,
            surface_raised: Color::Reset,
            surface_overlay: Color::Reset,
            surface_dimmed: Color::Reset,
            text_primary: Color::Reset,
            text_secondary: Color::Reset,
            text_tertiary: Color::Reset,
            text_accent: Color::Reset,
            hairline_default: Color::Reset,
            hairline_focus: Color::Reset,
            status_success: Color::Reset,
            status_warning: Color::Reset,
            status_error: Color::Reset,
            status_working: Color::Reset,
            status_idle: Color::Reset,
            monochrome: true,
        }
    }

    #[cfg(test)]
    pub fn truecolor_for_test() -> Self {
        Self::truecolor()
    }

    fn truecolor() -> Self {
        Self {
            surface_base: Color::Reset,
            surface_raised: rgb(0x1A, 0x1B, 0x20),
            surface_overlay: rgb(0x24, 0x25, 0x2B),
            surface_dimmed: Color::Black,
            text_primary: rgb(0xE2, 0xE2, 0xE2),
            text_secondary: rgb(0xA0, 0xA0, 0xA5),
            text_tertiary: rgb(0x60, 0x61, 0x65),
            text_accent: rgb(0x82, 0xA0, 0xFF),
            hairline_default: rgb(0x30, 0x31, 0x36),
            hairline_focus: rgb(0x82, 0xA0, 0xFF),
            status_success: rgb(0x86, 0xE5, 0x9A),
            status_warning: rgb(0xFF, 0xC2, 0x66),
            status_error: rgb(0xFF, 0x6B, 0x6B),
            status_working: rgb(0x82, 0xA0, 0xFF),
            status_idle: rgb(0x60, 0x61, 0x65),
            monochrome: false,
        }
    }

    fn ansi16() -> Self {
        Self {
            surface_base: Color::Reset,
            surface_raised: Color::Black,
            surface_overlay: Color::DarkGray,
            surface_dimmed: Color::Black,
            text_primary: Color::White,
            text_secondary: Color::Gray,
            text_tertiary: Color::DarkGray,
            text_accent: Color::LightBlue,
            hairline_default: Color::DarkGray,
            hairline_focus: Color::LightBlue,
            status_success: Color::LightGreen,
            status_warning: Color::Yellow,
            status_error: Color::LightRed,
            status_working: Color::LightBlue,
            status_idle: Color::DarkGray,
            monochrome: false,
        }
    }

    pub fn text_primary(self) -> Style {
        Style::default()
            .fg(self.text_primary)
            .add_modifier(Modifier::BOLD)
    }

    pub fn text_secondary(self) -> Style {
        Style::default().fg(self.text_secondary)
    }

    pub fn text_tertiary(self) -> Style {
        let style = Style::default().fg(self.text_tertiary);
        if self.monochrome {
            style.add_modifier(Modifier::DIM)
        } else {
            style
        }
    }

    pub fn text_accent(self) -> Style {
        Style::default().fg(self.text_accent)
    }

    pub fn surface_raised(self) -> Style {
        Style::default().bg(self.surface_raised)
    }

    pub fn surface_overlay(self) -> Style {
        Style::default().bg(self.surface_overlay)
    }

    pub fn surface_dimmed(self) -> Style {
        Style::default().bg(self.surface_dimmed)
    }

    pub fn hairline(self, focused: bool) -> Style {
        Style::default().fg(if focused {
            self.hairline_focus
        } else {
            self.hairline_default
        })
    }

    pub fn status_success(self) -> Style {
        Style::default().fg(self.status_success)
    }

    pub fn status_warning(self) -> Style {
        Style::default().fg(self.status_warning)
    }

    pub fn status_error(self) -> Style {
        Style::default().fg(self.status_error)
    }

    pub fn status_working(self) -> Style {
        Style::default().fg(self.status_working)
    }

    pub fn status_idle(self) -> Style {
        Style::default().fg(self.status_idle)
    }
}

fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb(r, g, b)
}

fn no_color_set() -> bool {
    std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty())
}

fn supports_truecolor() -> bool {
    match std::env::var("COLORTERM") {
        Ok(v) => {
            let v = v.to_ascii_lowercase();
            v.contains("truecolor") || v.contains("24bit")
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ansi16_has_no_ad_hoc_cyan() {
        let t = Theme::ansi16();
        assert_ne!(t.text_accent, Color::Cyan);
        assert_ne!(t.hairline_focus, Color::Cyan);
        assert_eq!(t.text_accent, Color::LightBlue);
    }

    #[test]
    fn truecolor_uses_design_hex() {
        let t = Theme::truecolor();
        assert_eq!(t.text_primary, Color::Rgb(0xE2, 0xE2, 0xE2));
        assert_eq!(t.text_accent, Color::Rgb(0x82, 0xA0, 0xFF));
        assert_eq!(t.hairline_default, Color::Rgb(0x30, 0x31, 0x36));
        assert_eq!(t.status_error, Color::Rgb(0xFF, 0x6B, 0x6B));
    }

    #[test]
    fn monochrome_uses_reset_only() {
        let t = Theme::monochrome();
        assert!(t.monochrome);
        assert_eq!(t.text_primary, Color::Reset);
        assert_eq!(t.status_working, Color::Reset);
    }

    #[test]
    fn text_primary_is_bold() {
        let style = Theme::ansi16().text_primary();
        assert!(style.add_modifier.contains(Modifier::BOLD));
    }
}
