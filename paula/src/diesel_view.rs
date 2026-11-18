//! ratatui widget for Diesel (feature `tui`).

use crate::diesel::{half_blocks, Image};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Widget;

/// Draws one frame with half blocks, clipped to the area.
pub struct DieselWidget<'a> {
    pub frame: &'a Image,
}

impl Widget for DieselWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        for (r, row) in half_blocks(self.frame).iter().enumerate() {
            for (c, cell) in row.iter().enumerate() {
                if c as u16 >= area.width || r as u16 >= area.height {
                    continue;
                }
                let mut style = Style::default();
                if let Some([r, g, b]) = cell.fg {
                    style = style.fg(Color::Rgb(r, g, b));
                }
                if let Some([r, g, b]) = cell.bg {
                    style = style.bg(Color::Rgb(r, g, b));
                }
                buf.set_string(area.x + c as u16, area.y + r as u16, cell.ch.to_string(), style);
            }
        }
    }
}
