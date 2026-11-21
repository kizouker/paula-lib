use super::{cyan, dim, green, pink};
use crate::audio::AudioBackend;
use crate::state::{Action, AppState};
use egui::{RichText, ScrollArea, Ui};

pub fn draw(
    ui: &mut Ui,
    app: &mut AppState,
    stream: &mut Option<cpal::Stream>,
    audio: &dyn AudioBackend,
) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("scores").strong().color(super::rgb((0, 0, 0))).background_color(cyan()));
        if let Some(menu) = app.menu.as_ref() {
            ui.label(format!("kit: {}", menu.kit().label()));
        }
        if ui.button("Kit").clicked() { app.dispatch(Action::NextKit, audio); }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Open .mod...").clicked() { app.open_requested = true; }
        });
    });

    ui.separator();
    ui.label(RichText::new("Scores (*.paula.toml)").color(cyan()));
    if let Some(menu) = app.menu.as_mut() {
        if menu.scores().is_empty() {
            ui.label(RichText::new("no *.paula.toml files here; try --menu <folder> or Open .mod...").color(pink()));
        } else {
            let scores: Vec<_> = menu.scores().to_vec();
            let selected = menu.selected();
            for (index, path) in scores.iter().enumerate() {
                let text = if index == selected { format!("> {}", path.display()) } else { format!("  {}", path.display()) };
                let response = ui.add(egui::Button::new(RichText::new(text).color(if index == selected { cyan() } else { dim() })).fill(if index == selected { super::rgb((0x10, 0x18, 0x28)) } else { super::rgb((0x0a, 0x0e, 0x14)) }));
                if response.clicked() {
                    app.dispatch(Action::Select(index as i32 - selected as i32), audio);
                }
            }
        }
    }

    ui.separator();
    let report = app.menu.as_ref().and_then(|menu| menu.lint_report());
    let diagnostics: Vec<String> = report.map(|r| r.lines()).unwrap_or_default();
    let has_errors = report.is_some_and(|r| r.has_errors());
    let clean = report.is_some() && diagnostics.is_empty();
    let border = if has_errors { pink() } else if clean { green() } else { dim() };
    egui::Frame::group(ui.style()).stroke(egui::Stroke::new(1.0_f32, border)).show(ui, |ui| {
        ui.set_min_height(120.0);
        ui.set_min_width(ui.available_width());
        ScrollArea::vertical()
            .auto_shrink([false, true])
            .vertical_scroll_offset(app.lint_scroll as f32 * 18.0)
            .show(ui, |ui| {
            if diagnostics.is_empty() {
                let text = if clean { "no findings" } else { "l lints the selected score" };
                ui.label(RichText::new(text).color(if clean { green() } else { dim() }));
            } else {
                for line in &diagnostics {
                    let color = if line.contains("error:") || line.contains("warning:") { pink() } else { dim() };
                    ui.label(RichText::new(line).color(color).monospace());
                }
            }
        });
    });

    ui.separator();
    ui.horizontal(|ui| {
        if ui.button("l Lint").clicked() { app.dispatch(Action::Lint, audio); }
        if ui.button("Enter Compile & Play").clicked() {
            *stream = None;
            app.dispatch(Action::Compile, audio);
        }
        if ui.button("w Save .mod + .wav").clicked() { app.dispatch(Action::Save, audio); }
        if ui.button("q Quit").clicked() { app.exit_requested = true; }
    });
    ui.label(RichText::new(format!("status: {}", app.status)).color(dim()).monospace());
}
