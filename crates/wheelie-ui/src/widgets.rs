//! Small shared widgets.

use eframe::egui;

/// Visual group used for settings cards.
pub fn card(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::same(16))
        .show(ui, add_contents);
}
