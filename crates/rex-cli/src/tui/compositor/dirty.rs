//! Cell-level damage tracking for dirty-rect diffing.

use ratatui::buffer::{Buffer, Cell};
use ratatui::layout::Rect;

#[derive(Debug, Default)]
pub struct DirtyTracker {
    prev: Vec<Cell>,
    area: Rect,
}

impl DirtyTracker {
    pub fn snapshot(&mut self, buf: &Buffer) {
        let area = buf.area;
        let len = (area.width as usize) * (area.height as usize);
        if self.prev.len() != len || self.area != area {
            self.prev = vec![Cell::default(); len];
            self.area = area;
        }
        for y in 0..area.height {
            for x in 0..area.width {
                let idx = (y as usize) * (area.width as usize) + (x as usize);
                if let Some(cell) = buf.cell((x, y)) {
                    self.prev[idx] = cell.clone();
                }
            }
        }
    }

    pub fn damage_rects(&self, buf: &Buffer) -> Vec<Rect> {
        let area = buf.area;
        if self.prev.len() != (area.width as usize) * (area.height as usize) {
            return vec![area];
        }
        let mut rects = Vec::new();
        let mut in_damage = false;
        let mut start_y = 0u16;
        for y in 0..area.height {
            let row_damaged = (0..area.width).any(|x| self.cell_changed(buf, x, y));
            if row_damaged && !in_damage {
                in_damage = true;
                start_y = y;
            } else if !row_damaged && in_damage {
                rects.push(Rect::new(area.x, start_y, area.width, y - start_y));
                in_damage = false;
            }
        }
        if in_damage {
            rects.push(Rect::new(
                area.x,
                start_y,
                area.width,
                area.height.saturating_sub(start_y),
            ));
        }
        if rects.is_empty() {
            vec![area]
        } else {
            rects
        }
    }

    fn cell_changed(&self, buf: &Buffer, x: u16, y: u16) -> bool {
        let idx = (y as usize) * (buf.area.width as usize) + (x as usize);
        let cur = buf.cell((x, y));
        match (cur, self.prev.get(idx)) {
            (Some(c), Some(p)) => c != p,
            _ => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    #[test]
    fn detects_single_cell_change() {
        let area = Rect::new(0, 0, 4, 2);
        let mut buf = Buffer::empty(area);
        let mut tracker = DirtyTracker::default();
        tracker.snapshot(&buf);
        buf.cell_mut((1, 0)).unwrap().set_fg(Color::Red);
        let rects = tracker.damage_rects(&buf);
        assert!(!rects.is_empty());
    }
}
