//! eframe shell for Wheelie.
//!
//! This module owns the `eframe::App` implementation. Business state lives in
//! `wheelie-app` and is observed through `RuntimeHandle`.

use eframe::egui;
use std::time::Duration;
use wheelie_app::{AppEvent, RuntimeHandle, RuntimeSnapshot};
use wheelie_core::{APP_ID, APP_NAME, Language, PlatformInfo, TextKey, VERSION, tr};

use crate::{ThemeMode, settings, theme};

/// Runs the native eframe loop.
pub fn run(runtime: RuntimeHandle) -> anyhow::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1120.0, 720.0])
            .with_min_inner_size([860.0, 560.0])
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
    language: Language,
    theme_mode: ThemeMode,
    tab: settings::SettingsTab,
    snapshot: RuntimeSnapshot,
    platform: PlatformInfo,
}

impl WheelieApp {
    fn new(cc: &eframe::CreationContext<'_>, runtime: RuntimeHandle) -> Self {
        theme::apply(&cc.egui_ctx, ThemeMode::Dark);
        let snapshot = runtime.snapshot();
        let platform = runtime.platform_info();
        Self {
            runtime,
            language: Language::Fr,
            theme_mode: ThemeMode::Dark,
            tab: settings::SettingsTab::default(),
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
}

impl eframe::App for WheelieApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_events();
        theme::apply(ctx, self.theme_mode);
        // The runtime is polled through a channel; a low-frequency repaint keeps
        // the UI in sync until the event-loop integration lands in M3.
        ctx.request_repaint_after(Duration::from_millis(100));

        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(APP_NAME)
                        .size(24.0)
                        .strong()
                        .color(theme::ACCENT),
                );
                ui.label(egui::RichText::new(format!("v{VERSION}")).weak());

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::ComboBox::from_id_salt("language")
                        .selected_text(self.language.native_name())
                        .show_ui(ui, |ui| {
                            for language in Language::ALL {
                                ui.selectable_value(
                                    &mut self.language,
                                    language,
                                    language.native_name(),
                                );
                            }
                        });

                    egui::ComboBox::from_id_salt("theme")
                        .selected_text(theme_label(self.language, self.theme_mode))
                        .show_ui(ui, |ui| {
                            for mode in ThemeMode::ALL {
                                ui.selectable_value(
                                    &mut self.theme_mode,
                                    mode,
                                    theme_label(self.language, mode),
                                );
                            }
                        });
                });
            });
            ui.add_space(6.0);
        });

        egui::SidePanel::left("navigation")
            .resizable(false)
            .exact_width(190.0)
            .show(ctx, |ui| {
                settings::navigation(ui, self.language, &mut self.tab);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(8.0);
                settings::page(
                    ui,
                    self.language,
                    self.tab,
                    &self.runtime,
                    &self.snapshot,
                    self.platform,
                );
            });
        });
    }
}

fn theme_label(language: Language, mode: ThemeMode) -> &'static str {
    match mode {
        ThemeMode::Dark => tr(language, TextKey::DarkTheme),
        ThemeMode::Light => tr(language, TextKey::LightTheme),
    }
}
