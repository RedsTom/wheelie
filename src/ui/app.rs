//! eframe shell for Wheelie.
//!
//! This module owns the `eframe::App` implementation. Business state lives in
//! `crate::app` and is observed through `RuntimeHandle`.

use crate::app::{AppEvent, RuntimeHandle, RuntimeSnapshot};
use crate::core::{APP_ID, APP_NAME, PlatformInfo, TextKey, VERSION, tr};
use crate::ui::settings::{self, SettingsTab, UiPreferences};
use crate::ui::{theme, widgets};
use eframe::egui;
use egui::{Margin, RichText, Stroke};
use std::time::Duration;

/// Runs the native eframe loop.
pub fn run(runtime: RuntimeHandle) -> anyhow::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 760.0])
            .with_min_inner_size([960.0, 620.0])
            .with_app_id(APP_ID)
            .with_title(APP_NAME),
        ..Default::default()
    };

    eframe::run_native(
        APP_NAME,
        options,
        Box::new(move |cc| Ok(Box::new(WheelieApp::new(cc, runtime)))),
    )
    .map_err(|error| anyhow::anyhow!("eframe event loop failed: {error}"))?;

    Ok(())
}

struct WheelieApp {
    runtime: RuntimeHandle,
    preferences: UiPreferences,
    tab: SettingsTab,
    snapshot: RuntimeSnapshot,
    platform: PlatformInfo,
}

impl WheelieApp {
    fn new(cc: &eframe::CreationContext<'_>, runtime: RuntimeHandle) -> Self {
        let preferences = UiPreferences::default();
        theme::install_fonts(&cc.egui_ctx);
        theme::apply(&cc.egui_ctx, preferences.theme_mode);

        let snapshot = runtime.snapshot();
        let platform = runtime.platform_info();
        Self {
            runtime,
            preferences,
            tab: SettingsTab::default(),
            snapshot,
            platform,
        }
    }

    fn drain_events(&mut self) {
        while let Some(event) = self.runtime.try_recv_event() {
            match event {
                AppEvent::StateChanged(snapshot) => self.snapshot = snapshot,
                AppEvent::Error(message) => self.snapshot.last_error = Some(message),
            }
        }
    }

    fn sidebar(&mut self, ui: &mut egui::Ui) {
        let language = self.preferences.language;

        ui.horizontal(|ui| {
            widgets::wheel_logo(ui, 24.0, theme::ACCENT);
            ui.add_space(2.0);
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(APP_NAME)
                        .font(theme::heading_font(17.0))
                        .color(theme::card_text(ui)),
                );
                ui.label(
                    RichText::new(tr(language, TextKey::AppTagline))
                        .size(10.5)
                        .color(theme::muted(ui)),
                );
            });
        });

        ui.add_space(24.0);
        ui.label(
            RichText::new(tr(language, TextKey::Workspace))
                .size(10.5)
                .color(theme::muted(ui)),
        );
        ui.add_space(4.0);
        settings::navigation(ui, language, &mut self.tab);

        ui.add_space((ui.available_height() - 118.0).max(12.0));

        let profile = self
            .snapshot
            .active_profile
            .as_deref()
            .unwrap_or_else(|| tr(language, TextKey::NoProfileLabel));
        let (state, state_color) = settings::state_label(language, self.snapshot.state);

        egui::Frame::NONE
            .fill(ui.visuals().faint_bg_color)
            .stroke(Stroke::new(1.0_f32, theme::border(ui)))
            .corner_radius(8)
            .inner_margin(10)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        widgets::status_pill(ui, state, state_color);
                    });
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(tr(language, TextKey::ActiveProfileLabel))
                            .size(10.0)
                            .color(theme::muted(ui)),
                    );
                    ui.label(
                        RichText::new(profile)
                            .size(12.0)
                            .color(theme::card_text(ui)),
                    );
                });
            });

        ui.add_space(8.0);
        ui.label(
            RichText::new(format!("v{VERSION} · GPL-3.0"))
                .size(10.5)
                .color(theme::muted(ui)),
        );
    }
}

impl eframe::App for WheelieApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_events();
        theme::apply(ctx, self.preferences.theme_mode);
        // The runtime is polled through a channel; a low-frequency repaint keeps
        // the UI in sync until the event-loop integration lands in M3.
        ctx.request_repaint_after(Duration::from_millis(100));

        egui::SidePanel::left("sidebar")
            .resizable(false)
            .exact_width(212.0)
            .frame(
                egui::Frame::NONE
                    .fill(theme::sidebar_fill(self.preferences.theme_mode))
                    .inner_margin(Margin::symmetric(16, 18)),
            )
            .show(ctx, |ui| self.sidebar(ui));

        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(theme::background(self.preferences.theme_mode))
                    .inner_margin(Margin::symmetric(28, 22)),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        settings::page(
                            ui,
                            &mut self.preferences,
                            self.tab,
                            &self.runtime,
                            &self.snapshot,
                            self.platform,
                        );
                    });
            });
    }
}
