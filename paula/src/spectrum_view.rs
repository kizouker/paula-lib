//! ratatui widget for the four-channel spectrum (feature `tui`).

use crate::spectrum::{Spectra, BARS, SPECTRUM_BANDS};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Widget;

/// Glow palette colours per channel: cyan, pink, green, orange.
pub const CHANNEL_COLORS: [Color; 4] = [
    Color::Rgb(0x00, 0xe5, 0xff),
    Color::Rgb(0xff, 0x2e, 0x97),
    Color::Rgb(0x39, 0xff, 0x88),
    Color::Rgb(0xff, 0xb0, 0x2e),
];
const DIM: Color = Color::Rgb(0x60, 0x6a, 0x80);

/// Four columns side by side. Each has a title row (`CH1` and its level in %) and bars below.
/// A bar is as high as the band level: whole rows of `█` plus one `▁..▇` cell on top; empty
/// cells show `·`. If a column is too narrow for 16 bands, neighbouring bands are merged (maximum).
pub struct SpectrumWidget<'a> {
    pub spectra: &'a Spectra,
}

impl Widget for SpectrumWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 2 {
            return;
        }
        let col_w = area.width / 4;
        let bar_h = (area.height - 1) as usize;
        for ch in 0..4 {
            let x0 = area.x + ch as u16 * col_w;
            let color = CHANNEL_COLORS[ch];
            let title = format!("CH{} {:3.0}%", ch + 1, self.spectra.levels[ch] * 100.0);
            buf.set_string(x0, area.y, &title, Style::default().fg(color));
            let cols = ((col_w.saturating_sub(1)) as usize).clamp(1, SPECTRUM_BANDS);
            for c in 0..cols {
                let lo = c * SPECTRUM_BANDS / cols;
                let hi = ((c + 1) * SPECTRUM_BANDS / cols).max(lo + 1).min(SPECTRUM_BANDS);
                let level = self.spectra.bands[ch][lo..hi].iter().cloned().fold(0.0f32, f32::max);
                let eighths = (level.clamp(0.0, 1.0) * (bar_h * 8) as f32).round() as usize;
                for r in 0..bar_h {
                    // r = 0 is the bottom row
                    let here = eighths.saturating_sub(r * 8).min(8);
                    let y = area.y + area.height - 1 - r as u16;
                    let x = x0 + c as u16;
                    if here == 0 {
                        buf.set_string(x, y, "·", Style::default().fg(DIM));
                    } else {
                        buf.set_string(x, y, BARS[here - 1].to_string(), Style::default().fg(color));
                    }
                }
            }
        }
    }
}
