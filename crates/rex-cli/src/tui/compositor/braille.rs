//! Braille pattern glyphs for flux and carousel scale effects.

const BRAILLE_BASE: u32 = 0x2800;

/// Braille dot pattern for flux sweep position (0..7 within one cell cycle).
pub fn flux_glyph(phase: u8) -> char {
    let dots = match phase % 8 {
        0 => 0x01,
        1 => 0x03,
        2 => 0x07,
        3 => 0x0F,
        4 => 0x1E,
        5 => 0x3C,
        6 => 0x78,
        _ => 0xF0,
    };
    char::from_u32(BRAILLE_BASE + dots).unwrap_or('⠿')
}

/// Half-block scale for carousel adjacent sessions.
pub fn carousel_adjacent_glyph(fade: f32) -> char {
    if fade < 0.33 {
        '▁'
    } else if fade < 0.66 {
        '▄'
    } else {
        '▆'
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flux_glyphs_are_braille_range() {
        for i in 0..8 {
            let c = flux_glyph(i);
            assert!(('\u{2800}'..='\u{28FF}').contains(&c));
        }
    }
}
