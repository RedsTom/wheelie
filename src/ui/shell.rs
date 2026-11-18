//! gpui application shell for Wheelie.
//!
//! Boots the gpui application, owns the settings window and runs the 35 ms
//! poll tick that replaces the eframe `update()` body: runtime events, OS
//! services (tray, global keys), gesture detection and repaints. The old
//! eframe shell in `app.rs` keeps compiling until the port is complete (F4).
//!
//! Close-to-tray: gpui exposes no window hide, so the settings window is
//! removed on close and recreated by the tray Show command while
//! `QuitMode::Explicit` keeps the process alive.

use crate::app::{RuntimeHandle, RuntimeSnapshot, RuntimeState};
use crate::core::{
    APP_NAME, Config, GestureConfig, GestureDetector, Language, PlatformInfo, TextKey, VERSION, tr,
};
use crate::platform::{GlobalHotkeyService, InjectionGuard};
use crate::ui::overlay::OverlayState;
use crate::ui::pages::PageState;
use crate::ui::settings::{SettingsTab, UiPreferences};
use crate::ui::{assets, gestures, pages, services, style, window};
use gpui::prelude::*;
use gpui::{
    App, AsyncApp, Bounds, Context, Pixels, QuitMode, Render, Window, WindowHandle, div, px,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;
use tracing::info;

/// Shared shell state: mutated by the tick, read by the settings view.
pub struct ShellState {
    /// Runtime handle used to read and persist configuration and actions.
    pub runtime: RuntimeHandle,
    /// UI language and theme.
    pub preferences: UiPreferences,
    /// Selected settings page.
    pub tab: SettingsTab,
    /// Ephemeral page UI state (menus, buffers, focus).
    pub page: PageState,
    /// Latest runtime snapshot.
    pub snapshot: RuntimeSnapshot,
    /// Platform capabilities.
    pub platform: PlatformInfo,
    /// Gesture detector fed by polled hotkeys.
    pub detector: GestureDetector,
    /// Detector configuration, kept in sync with the config.
    pub gesture_config: GestureConfig,
    /// Injection guard used to drop self-injected keys.
    pub injection: InjectionGuard,
    /// Global key watcher.
    pub hotkeys: Option<GlobalHotkeyService>,
    /// Watcher creation failed.
    pub hotkeys_failed: bool,
    /// Bindings changed; the watcher must re-register.
    pub hotkeys_dirty: bool,
    /// Runtime paused or stopped.
    pub paused: bool,
    /// Control currently being key-tested.
    pub testing: Option<crate::core::Control>,
    /// Last chord received during a key test.
    pub last_key: Option<crate::core::KeyChord>,
    /// Radial wheel selection state (window lands in F3).
    pub overlay: OverlayState,
    /// Tray icon service.
    pub tray: Option<crate::platform::TrayService>,
    /// Language the tray icon was built with.
    pub tray_language: Language,
    /// Tray creation failed.
    pub tray_failed: bool,
    /// Settings window removed (close-to-tray).
    pub hidden: bool,
    /// Quit requested.
    pub quitting: bool,
    /// Open settings window handle.
    pub settings_window: Option<WindowHandle<SettingsView>>,
    /// Last known settings bounds, restored on recreate.
    pub settings_bounds: Option<Bounds<Pixels>>,
}

/// Shared handle type.
pub type Shared = Rc<RefCell<ShellState>>;

impl ShellState {
    fn new(runtime: RuntimeHandle, cx: &App) -> Self {
        let config = runtime.config();
        let preferences = UiPreferences {
            language: config.language,
            theme_mode: config.general.theme,
        };

        let gesture_config = config.general.gesture_config();
        let bindings = config
            .active_profile()
            .map(|profile| profile.bindings)
            .unwrap_or_else(|| Config::default().profiles[0].bindings);

        let snapshot = runtime.snapshot();
        let platform = runtime.platform_info();
        let injection = runtime.injection_guard();

        Self {
            runtime,
            preferences,
            tab: SettingsTab::default(),
            page: PageState::new(cx),
            snapshot,
            platform,
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
            tray: None,
            tray_language: preferences.language,
            tray_failed: false,
            hidden: false,
            quitting: false,
            settings_window: None,
            settings_bounds: None,
        }
    }
}

/// Root view of the settings window.
pub struct SettingsView {
    /// Shared shell state.
    pub shared: Shared,
}

impl SettingsView {
    fn state_pill(&self, state: &ShellState) -> (&'static str, gpui::Rgba) {
        let language = state.preferences.language;
        match state.snapshot.state {
            RuntimeState::Starting => (tr(language, TextKey::StateStarting), style::WARNING),
            RuntimeState::Running => (tr(language, TextKey::StateRunning), style::SUCCESS),
            RuntimeState::Paused => (tr(language, TextKey::StatePaused), style::WARNING),
            RuntimeState::Stopped => (tr(language, TextKey::StateStopped), style::DANGER),
        }
    }

    fn sidebar(&self, cx: &mut Context<Self>, state: &ShellState) -> impl IntoElement {
        let t = style::tokens(state.preferences.theme_mode);
        let language = state.preferences.language;
        let shared = self.shared.clone();
        let weak = cx.weak_entity();

        let nav = SettingsTab::ALL
            .iter()
            .map(|tab| {
                let selected = state.tab == *tab;
                let tab = *tab;
                let shared = shared.clone();
                let weak = weak.clone();
                div()
                    .id(tab.label(language))
                    .w_full()
                    .flex()
                    .items_center()
                    .px_3()
                    .py(px(7.0))
                    .rounded(px(style::RADIUS_CONTROL))
                    .when(selected, |div| div.bg(t.accent_soft))
                    .when(selected, |div| div.text_color(style::ACCENT))
                    .when(!selected, |div| div.text_color(t.muted))
                    .hover(move |div| if selected { div } else { div.bg(t.card_hover) })
                    .cursor_pointer()
                    .child(tab.label(language))
                    .on_click(move |_, _, cx| {
                        shared.borrow_mut().tab = tab;
                        if let Some(view) = weak.upgrade() {
                            view.update(cx, |_, cx| cx.notify());
                        }
                    })
            })
            .collect::<Vec<_>>();

        let (state_label, state_color) = self.state_pill(state);
        let profile = services::active_set_name(state);

        div()
            .flex_none()
            .w(px(style::SIDEBAR_WIDTH))
            .h_full()
            .flex()
            .flex_col()
            .bg(t.sidebar)
            .px_4()
            .pt(px(18.0))
            .pb_4()
            .gap_1()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .mb_5()
                    .child(
                        div()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_size(px(17.0))
                            .text_color(t.text)
                            .child(APP_NAME),
                    )
                    .child(
                        div()
                            .text_size(px(10.5))
                            .text_color(t.muted)
                            .child(tr(language, TextKey::AppTagline)),
                    ),
            )
            .child(
                div()
                    .text_size(px(10.5))
                    .text_color(t.muted)
                    .mb_1()
                    .child(tr(language, TextKey::Workspace)),
            )
            .children(nav)
            .child(div().flex_1())
            .child(
                div()
                    .w_full()
                    .rounded(px(style::RADIUS_CARD))
                    .border_1()
                    .border_color(t.border)
                    .bg(t.card)
                    .p(px(10.0))
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .px_2()
                            .py(px(1.0))
                            .rounded_full()
                            .text_size(px(11.0))
                            .text_color(state_color)
                            .bg(gpui::Rgba {
                                a: 0.16,
                                ..state_color
                            })
                            .child(state_label),
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(t.muted)
                            .child(tr(language, TextKey::ActiveSetLabel)),
                    )
                    .child(div().text_size(px(12.0)).text_color(t.text).child(profile)),
            )
            .child(
                div()
                    .mt_2()
                    .text_size(px(10.5))
                    .text_color(t.muted)
                    .child(format!("v{VERSION} · GPL-3.0")),
            )
    }
}

impl Render for SettingsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.shared.borrow();
        let t = style::tokens(state.preferences.theme_mode);
        let sidebar = self.sidebar(cx, &state);

        div()
            .size_full()
            .flex()
            .font_family("Rubik")
            .text_size(px(14.0))
            .text_color(t.text)
            .bg(t.bg)
            .child(sidebar)
            .child(
                div()
                    .id("page-scroll")
                    .flex_1()
                    .min_w(px(0.0))
                    .h_full()
                    .overflow_scroll()
                    .bg(t.bg)
                    .px(px(28.0))
                    .py(px(22.0))
                    .child(pages::render(&state, &self.shared, window, cx)),
            )
    }
}

/// Runs the gpui application. Called from `main` via `ui::run`.
pub fn run(runtime: RuntimeHandle) -> anyhow::Result<()> {
    gpui_platform::application()
        .with_assets(assets::WheelieAssets)
        .run(move |cx: &mut App| {
            cx.set_quit_mode(QuitMode::Explicit);

            if let Err(error) = assets::install_fonts(cx) {
                eprintln!("failed to load bundled fonts: {error}");
                return;
            }

            info!("gpui shell starting");
            let shared: Shared = Rc::new(RefCell::new(ShellState::new(runtime, cx)));
            window::open_settings(cx, &shared);

            let tick_shared = shared.clone();
            cx.spawn(async move |cx: &mut AsyncApp| {
                let mut sleep = Duration::from_millis(35);
                loop {
                    cx.background_executor().timer(sleep).await;
                    sleep = cx.update(|app| poll_tick(app, &tick_shared));
                    if sleep.is_zero() {
                        break;
                    }
                }
            })
            .detach();
        });
    Ok(())
}

/// One poll iteration; returns the sleep before the next tick (`ZERO` when
/// quitting). A pending double-press deadline shortens the sleep so the
/// 350 ms single-click window resolves on time.
fn poll_tick(cx: &mut App, shared: &Shared) -> Duration {
    {
        let mut state = shared.borrow_mut();
        services::drain_events(&mut state);
        services::persist_preferences(&mut state);
        services::ensure_services(&mut state);
        services::sync_from_config(&mut state);
    }

    let commands = {
        let mut state = shared.borrow_mut();
        services::poll_tray(&mut state)
    };
    let mut show = false;
    let mut hide = false;
    let mut quit = false;
    for command in &commands {
        match command {
            crate::platform::TrayCommand::Show => show = true,
            crate::platform::TrayCommand::Hide => hide = true,
            crate::platform::TrayCommand::Quit => quit = true,
            crate::platform::TrayCommand::Pause | crate::platform::TrayCommand::Resume => {}
        }
        let mut state = shared.borrow_mut();
        services::apply_tray_command(&mut state, command);
    }

    let quitting = shared.borrow().quitting;
    let mut expiry: Option<Duration> = None;
    if !quitting {
        {
            let mut state = shared.borrow_mut();
            services::apply_hotkeys_if_needed(&mut state);
            services::poll_hotkeys(&mut state);
            expiry = gestures::tick_detector(&mut state);
        }
        if show {
            window::focus_settings(cx, shared);
            shared.borrow_mut().hidden = false;
        }
        if hide {
            window::close_settings(cx, shared);
            shared.borrow_mut().hidden = true;
        }
    }

    window::reap_closed(cx, shared);
    window::notify_settings(cx, shared);

    if quit {
        cx.quit();
        return Duration::ZERO;
    }
    if shared.borrow().quitting {
        return Duration::ZERO;
    }
    expiry
        .unwrap_or(Duration::from_millis(35))
        .min(Duration::from_millis(35))
}
