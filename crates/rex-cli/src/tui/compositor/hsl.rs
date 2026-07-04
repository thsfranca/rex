//! HSL interpolation on truecolor semantic tokens.

use ratatui::style::Color;

pub fn lerp_rgb(a: (u8, u8, u8), b: (u8, u8, u8), t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    let mix = |x: u8, y: u8| -> u8 { (x as f32 + (y as f32 - x as f32) * t) as u8 };
    Color::Rgb(mix(a.0, b.0), mix(a.1, b.1), mix(a.2, b.2))
}

pub fn dim_luminosity(c: (u8, u8, u8), factor: f32) -> Color {
    let f = factor.clamp(0.0, 1.0);
    Color::Rgb(
        (c.0 as f32 * f) as u8,
        (c.1 as f32 * f) as u8,
        (c.2 as f32 * f) as u8,
    )
}

pub fn typing_glow(intensity: f32) -> Color {
    let i = intensity.clamp(0.0, 1.0);
    lerp_rgb((0x1A, 0x1B, 0x20), (0x82, 0xA0, 0xFF), i * 0.35)
}

pub fn diff_scrub_green(intensity: f32) -> Color {
    lerp_rgb((0x1A, 0x1B, 0x20), (0x86, 0xE5, 0x9A), intensity)
}

pub fn diff_scrub_red(intensity: f32) -> Color {
    lerp_rgb((0x1A, 0x1B, 0x20), (0xFF, 0x6B, 0x6B), intensity)
}
