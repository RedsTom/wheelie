//! eframe shell for Wheelie.
//!
//! The shell owns the OS adapters that need the window event loop: the global
//! key watcher and the tray icon are created lazily on the first frame and
//! polled from here. Knob gestures are detected on this thread and only the
//! resulting actions are handed to the background worker.
//!
//! # Overlay approach
//!
//! The radial wheel borrows the root window: when the overlay opens the root
//! viewport is resized to OVERLAY_SIZE×OVERLAY_SIZE, stripped of decorations
//! and raised to always-on-top. The wgpu renderer clears with alpha=0, so
//! pixels outside the wheel are fully transparent. When the overlay closes the
//! saved geometry is restored. No secondary viewport is used because wgpu does
//! not alpha-composite secondary viewports on Windows.

use crate::app::action_worker::discard_injected;
use crate::app::{AppCommand, AppEvent, RuntimeHandle, RuntimeSnapshot, RuntimeState};
use crate::assets;
use crate::core::{
    APP_ID, APP_NAME, Config, Control, GestureConfig, GestureDetector, GestureEvent, InputEvent,
    KeyChord, Language, PlatformInfo, TextKey, VERSION, tr,
};
use crate::platform::{
    GlobalHotkeyService, HotkeySource, InjectionGuard, TrayCommand, TrayService,
};
use crate::ui::overlay::{self, OVERLAY_SIZE, OverlayOutcome, OverlayState};
use crate::ui::settings::{self, PageContext, SettingsTab, UiPreferences};
use crate::ui::{theme, widgets};
use eframe::egui;
use egui::{Margin, RichText, Stroke, Vec2, ViewportCommand};
use std::time::{Duration, Instant};
use tracing::{info, warn};

/// Runs the native eframe loop.
pub fn run(runtime: RuntimeHandle) -> anyhow::Result<()> {
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 760.0])
            .with_min_inner_size([960.0, 620.0])
            .with_app_id(APP_ID)
            .with_transparent(true)
            .with_icon(assets::window_icon())
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

/// Saved window state so we can restore it after the overlay closes.
struct SavedWindow {
    size: Vec2,
}

struct WheelieApp {
    runtime: RuntimeHandle,
    preferences: UiPreferences,
    tab: SettingsTab,
    snapshot: RuntimeSnapshot,
    platform: PlatformInfo,
    logo: egui::TextureHandle,
    detector: GestureDetector,
    gesture_config: GestureConfig,
    injection: InjectionGuard,
    hotkeys: Option<GlobalHotkeyService>,
    hotkeys_failed: bool,
    hotkeys_dirty: bool,
    paused: bool,
    testing: Option<Control>,
    last_key: Option<KeyChord>,
    overlay: OverlayState,
    /// Geometry saved when the overlay borrows the root window.
    wheel_saved: Option<SavedWindow>,
    tray: Option<TrayService>,
    tray_language: Language,
    tray_failed: bool,
    hidden: bool,
    quitting: bool,
}

impl WheelieApp {
    fn new(cc: &eframe::CreationContext<'_>, runtime: RuntimeHandle) -> Self {
        let config = runtime.config();
        let preferences = UiPreferences {
            language: config.language,
            theme_mode: config.general.theme,
        };
        theme::install_fonts(&cc.egui_ctx);
        theme::apply(&cc.egui_ctx, preferences.theme_mode);

        let gesture_config = config.general.gesture_config();
        let bindings = config
            .active_profile()
            .map(|profile| profile.bindings)
            .unwrap_or_else(|| Config::default().profiles[0].bindings);

        let snapshot = runtime.snapshot();
        let platform = runtime.platform_info();
        let logo = assets::load_logo_texture(&cc.egui_ctx);
        let injection = runtime.injection_guard();

        Self {
            runtime,
            preferences,
            tab: SettingsTab::default(),
            snapshot,
            platform,
            logo,
            detector: GestureDetector::new(bindings, gesture_config),
            gesture_config,
            injection,
            hotkeys: None,
            hotkeys_failed: false,
            hotkeys_dirty: true,
            paused: false,
            testing: None,
            last_key: None,
            overlay: OverlayState::default(),
            wheel_saved: None,
            tray: None,
            tray_language: preferences.language,
            tray_failed: false,
            hidden: false,
            quitting: false,
        }
    }

    fn drain_events(&mut self) {
        while let Some(event) = self.runtime.try_recv_event() {
            match event {
                AppEvent::StateChanged(snapshot) => self.snapshot = snapshot,
                AppEvent::Error(message) => self.snapshot.last_error = Some(message),
                AppEvent::Indicator(_) => {}
            }
        }
    }

    /// Creates the OS services that need the window event loop.
    fn ensure_services(&mut self) {
        if self.hotkeys.is_none() && !self.hotkeys_failed {
            match GlobalHotkeyService::new() {
                Ok(service) => {
                    info!("global key watcher ready");
                    self.hotkeys = Some(service);
                    self.hotkeys_dirty = true;
                }
                Err(error) => {
                    warn!(%error, "global key watcher unavailable");
                    self.hotkeys_failed = true;
                    self.runtime.report_error(error.to_string());
                }
            }
        }

        if self.tray.is_none() && !self.tray_failed && self.runtime.config().general.show_tray {
            let language = self.preferences.language;
            let built = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                TrayService::new(language)
            }));
            match built {
                Ok(Ok(tray)) => {
                    self.tray = Some(tray);
                    self.tray_language = language;
                }
                Ok(Err(error)) => {
                    warn!(%error, "tray icon unavailable");
                    self.tray_failed = true;
                }
                Err(_) => {
                    warn!("tray icon unavailable on this system");
                    self.tray_failed = true;
                }
            }
        }

        if self.tray_language != self.preferences.language
            && let Some(tray) = self.tray.take()
        {
            drop(tray);
            self.tray_failed = false;
        }
    }

    /// Keeps the gesture detector aligned with the active profile.
    fn sync_from_config(&mut self) {
        let config = self.runtime.config();

        if let Some(profile) = config.active_profile()
            && profile.bindings != self.detector.bindings()
        {
            self.detector.set_bindings(profile.bindings);
            self.hotkeys_dirty = true;
        }

        let gesture_config = config.general.gesture_config();
        if gesture_config != self.gesture_config {
            let bindings = self.detector.bindings();
            let overlay_open = self.overlay.open;
            self.detector = GestureDetector::new(bindings, gesture_config);
            self.detector.set_overlay_open(overlay_open);
            self.gesture_config = gesture_config;
        }

        let paused = matches!(
            self.snapshot.state,
            RuntimeState::Paused | RuntimeState::Stopped
        );
        if paused != self.paused {
            self.paused = paused;
            self.hotkeys_dirty = true;
        }
    }

    fn apply_hotkeys_if_needed(&mut self) {
        if !self.hotkeys_dirty {
            return;
        }
        self.hotkeys_dirty = false;

        let Some(service) = &mut self.hotkeys else {
            return;
        };

        let result = if self.paused {
            service.unregister_all()
        } else if let Some(control) = self.testing {
            service.apply(&[self.detector.bindings().chord_for(control)])
        } else {
            let bindings = self.detector.bindings();
            service.apply(&[bindings.left, bindings.right, bindings.press])
        };

        match result {
            Ok(()) => self
                .runtime
                .set_capture_active(!self.paused && self.testing.is_none()),
            Err(error) => {
                self.runtime.set_capture_active(false);
                self.runtime.report_error(error.to_string());
            }
        }
    }

    fn poll_hotkeys(&mut self) {
        let events = match &mut self.hotkeys {
            Some(service) => service.poll(),
            None => Vec::new(),
        };
        let events = discard_injected(events, &self.injection);

        if events.is_empty() {
            return;
        }

        if let Some(_control) = self.testing {
            for event in &events {
                if let InputEvent::Pressed(chord) = event {
                    self.last_key = Some(*chord);
                }
            }
            return;
        }

        if self.paused {
            return;
        }

        let now = Instant::now();
        for event in events {
            for gesture in self.detector.feed(event, now) {
                self.handle_gesture(gesture);
            }
        }
    }

    fn tick_detector(&mut self, ctx: &egui::Context) {
        let now = Instant::now();
        for gesture in self.detector.tick(now) {
            self.handle_gesture(gesture);
        }

        if let Some(expiry) = self.detector.pending_expiry() {
            let remaining = expiry.saturating_duration_since(now);
            ctx.request_repaint_after(remaining.max(Duration::from_millis(5)));
        }
    }

    fn handle_gesture(&mut self, gesture: GestureEvent) {
        match gesture {
            GestureEvent::DoublePress => self.open_overlay(),
            GestureEvent::RotateLeft => {
                if self.overlay.open {
                    let count = self.runtime.config().action_sets.len();
                    self.overlay.select_previous(count);
                } else {
                    self.execute(Control::Left);
                }
            }
            GestureEvent::RotateRight => {
                if self.overlay.open {
                    let count = self.runtime.config().action_sets.len();
                    self.overlay.select_next(count);
                } else {
                    self.execute(Control::Right);
                }
            }
            GestureEvent::SinglePress => {
                if self.overlay.open {
                    self.confirm_overlay();
                } else {
                    self.execute(Control::Press);
                }
            }
        }
    }

    fn open_overlay(&mut self) {
        if self.overlay.open {
            return;
        }
        let config = self.runtime.config();
        let count = config.action_sets.len();
        if count == 0 {
            return;
        }
        let index = config
            .active_set
            .as_deref()
            .and_then(|id| config.action_sets.iter().position(|set| set.id == id))
            .unwrap_or(0);
        self.overlay.open_on(index, count);
        self.detector.set_overlay_open(true);
    }

    fn close_overlay(&mut self) {
        if !self.overlay.open {
            return;
        }
        self.overlay.open = false;
        self.overlay.outcome = None;
        self.detector.set_overlay_open(false);
    }

    fn confirm_overlay(&mut self) {
        let index = self.overlay.selected;
        self.activate_set(index);
        self.close_overlay();
    }

    /// Makes the set at `index` the active one. The wheel never runs actions.
    fn activate_set(&mut self, index: usize) {
        let Some(id) = self
            .runtime
            .config()
            .action_sets
            .get(index)
            .map(|set| set.id.clone())
        else {
            return;
        };
        if let Err(error) = self
            .runtime
            .update_config(|config| config.active_set = Some(id))
        {
            self.runtime.report_error(error.to_string());
        }
    }

    fn execute(&mut self, control: Control) {
        let config = self.runtime.config();
        let Some(set) = config.active_set_or_first() else {
            return;
        };
        let action = set.actions.for_control(control).clone();
        if let Err(error) = self.runtime.execute(action) {
            self.runtime.report_error(error.to_string());
        }
    }

    fn persist_preferences(&mut self) {
        let config = self.runtime.config();
        if config.language == self.preferences.language
            && config.general.theme == self.preferences.theme_mode
        {
            return;
        }

        let language = self.preferences.language;
        let theme_mode = self.preferences.theme_mode;
        if let Err(error) = self.runtime.update_config(|config| {
            config.language = language;
            config.general.theme = theme_mode;
        }) {
            self.runtime.report_error(error.to_string());
        }
    }

    fn handle_close_request(&mut self, ctx: &egui::Context) {
        if !ctx.input(|input| input.viewport().close_requested()) {
            return;
        }
        if self.quitting || !self.runtime.config().general.show_tray {
            self.quitting = true;
            return;
        }
        ctx.send_viewport_cmd(ViewportCommand::CancelClose);
        ctx.send_viewport_cmd(ViewportCommand::Visible(false));
        self.hidden = true;
    }

    fn poll_tray(&mut self, ctx: &egui::Context) {
        let commands = self
            .tray
            .as_ref()
            .map(TrayService::poll)
            .unwrap_or_default();

        for command in commands {
            match command {
                TrayCommand::Show => {
                    self.hidden = false;
                    ctx.send_viewport_cmd(ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd(ViewportCommand::Focus);
                }
                TrayCommand::Hide => {
                    self.hidden = true;
                    ctx.send_viewport_cmd(ViewportCommand::Visible(false));
                }
                TrayCommand::Pause => {
                    self.testing = None;
                    let _ = self.runtime.send(AppCommand::Pause);
                }
                TrayCommand::Resume => {
                    let _ = self.runtime.send(AppCommand::Resume);
                }
                TrayCommand::Quit => {
                    self.quitting = true;
                    let _ = self.runtime.send(AppCommand::Quit);
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }
            }
        }
    }

    fn active_set_name(&self) -> String {
        let config = self.runtime.config();
        let language = self.preferences.language;
        match config.active_set_or_first() {
            Some(set) => set.name.clone(),
            None => tr(language, TextKey::NoSetLabel).to_owned(),
        }
    }

    fn sidebar(&mut self, ui: &mut egui::Ui) {
        let language = self.preferences.language;

        ui.horizontal(|ui| {
            widgets::logo_image(ui, &self.logo, 30.0);
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

        let profile = self.active_set_name();
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
                        RichText::new(tr(language, TextKey::ActiveSetLabel))
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

    fn settings_panels(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("sidebar")
            .resizable(false)
            .exact_width(212.0)
            .frame(
                egui::Frame::NONE
                    .fill(theme::sidebar_fill(self.preferences.theme_mode))
                    .inner_margin(Margin::symmetric(16, 18)),
            )
            .show(ctx, |ui| self.sidebar(ui));

        let mut test_request = None;
        let mut bindings_changed = false;

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
                        let mut context = PageContext {
                            runtime: &self.runtime,
                            language: self.preferences.language,
                            platform: self.platform,
                            logo: &self.logo,
                            last_key: self.last_key,
                            testing: self.testing,
                            test_request: None,
                            bindings_changed: false,
                        };
                        settings::page(
                            ui,
                            &mut self.preferences,
                            self.tab,
                            &mut context,
                            &self.snapshot,
                        );
                        test_request = context.test_request;
                        bindings_changed = context.bindings_changed;
                    });
            });

        if let Some(request) = test_request {
            self.testing = request;
            self.last_key = None;
            self.hotkeys_dirty = true;
        }
        if bindings_changed {
            self.hotkeys_dirty = true;
        }
    }

    /// Morphs the root window into the radial-wheel overlay.
    ///
    /// Saves current geometry, removes decorations, resizes to OVERLAY_SIZE and
    /// raises to always-on-top. The wgpu renderer clears with alpha=0, so the
    /// transparent round wheel floats on the desktop.
    fn enter_wheel_mode(&mut self, ctx: &egui::Context) {
        if self.wheel_saved.is_some() {
            return; // already morphed
        }
        let info = ctx.input(|i| i.viewport().clone());
        let size = info
            .inner_rect
            .map(|r| r.size())
            .unwrap_or(Vec2::new(1180.0, 760.0));

        self.wheel_saved = Some(SavedWindow { size });

        ctx.send_viewport_cmd(ViewportCommand::Maximized(false));
        ctx.send_viewport_cmd(ViewportCommand::Decorations(false));
        ctx.send_viewport_cmd(ViewportCommand::WindowLevel(egui::WindowLevel::AlwaysOnTop));
        ctx.send_viewport_cmd(ViewportCommand::InnerSize(Vec2::splat(OVERLAY_SIZE)));
        ctx.send_viewport_cmd(ViewportCommand::Visible(true));
        if let Some(cmd) = ViewportCommand::center_on_screen(ctx) {
            ctx.send_viewport_cmd(cmd);
        }
    }

    /// Restores the root window to its settings-panel state.
    fn exit_wheel_mode(&mut self, ctx: &egui::Context) {
        let Some(saved) = self.wheel_saved.take() else {
            return;
        };
        ctx.send_viewport_cmd(ViewportCommand::WindowLevel(egui::WindowLevel::Normal));
        ctx.send_viewport_cmd(ViewportCommand::Decorations(true));
        ctx.send_viewport_cmd(ViewportCommand::InnerSize(saved.size));
    }

    /// Draws the wheel inside the (now 380×380, transparent) root window.
    fn draw_wheel(&mut self, ctx: &egui::Context) {
        let sets = self.runtime.config().action_sets;
        overlay::draw(ctx, &sets, &mut self.overlay);

        match self.overlay.outcome.take() {
            Some(OverlayOutcome::Close) => self.close_overlay(),
            Some(OverlayOutcome::Confirm(index)) => {
                self.activate_set(index);
                self.close_overlay();
            }
            None => {}
        }
    }
}

impl eframe::App for WheelieApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        // Always clear to fully transparent. When showing settings the panels
        // paint their own solid backgrounds. When showing the overlay the
        // wheel paints only its circular geometry; the rest is alpha=0.
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_events();
        self.persist_preferences();
        self.ensure_services();
        self.sync_from_config();
        self.handle_close_request(ctx);
        self.poll_tray(ctx);

        if !self.quitting {
            self.apply_hotkeys_if_needed();
            self.poll_hotkeys();
            self.tick_detector(ctx);
        }

        if self.overlay.open {
            self.enter_wheel_mode(ctx);
            self.draw_wheel(ctx);
        } else {
            self.exit_wheel_mode(ctx);
            // Only apply the theme when drawing settings; the wheel draws with
            // an explicit transparent background and does not need the theme.
            theme::apply(ctx, self.preferences.theme_mode);
            self.settings_panels(ctx);
        }

        // Keep polling while the window is hidden so knob input still works.
        ctx.request_repaint_after(Duration::from_millis(35));
    }
}
