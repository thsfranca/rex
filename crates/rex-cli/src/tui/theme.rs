//! Semantic style tokens for the terminal harness (R080).

use ratatui::style::{Color, Modifier, Style};

/// Palette used by the TUI. Colors enhance glyphs; monochrome remains usable.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub text: Color,
    pub text_muted: Color,
    pub text_bright: Color,
    pub accent: Color,
    pub border: Color,
    pub border_focus: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,
}

impl Theme {
    pub fn default_adaptive() -> Self {
        Self {
            text: Color::Reset,
            text_muted: Color::DarkGray,
            text_bright: Color::White,
            accent: Color::Cyan,
            border: Color::DarkGray,
            border_focus: Color::Cyan,
            success: Color::Green,
            warning: Color::Yellow,
            error: Color::Red,
            info: Color::Blue,
        }
    }

    pub fn text(self) -> Style {
        Style::default().fg(self.text)
    }

    pub fn muted(self) -> Style {
        Style::default().fg(self.text_muted)
    }

    pub fn bright(self) -> Style {
        Style::default()
            .fg(self.text_bright)
            .add_modifier(Modifier::BOLD)
    }

    pub fn accent(self) -> Style {
        Style::default().fg(self.accent)
    }

    pub fn success(self) -> Style {
        Style::default().fg(self.success)
    }

    pub fn warning(self) -> Style {
        Style::default().fg(self.warning)
    }

    pub fn error(self) -> Style {
        Style::default().fg(self.error)
    }

    pub fn info(self) -> Style {
        Style::default().fg(self.info)
    }

    pub fn border(self, focused: bool) -> Style {
        Style::default().fg(if focused {
            self.border_focus
        } else {
            self.border
        })
    }
}
