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

use crate::app::{RuntimeHandle, RuntimeSnapshot};
use crate::core::{
    APP_NAME, Config, GestureConfig, GestureDetector, Language, PlatformInfo, TextKey, ThemeMode,
    VERSION, tr,
};
use crate::platform::{GlobalHotkeyService, InjectionGuard};
use crate::ui::osd_window::{Osd, OsdView};
use crate::ui::overlay::OverlayState;
use crate::ui::overlay_window::OverlayView;
use crate::ui::pages::PageState;
use crate::ui::settings::{SettingsTab, UiPreferences};
use crate::ui::{assets, gestures, osd_window, overlay_window, pages, services, style, window};
use gpui::prelude::*;
use gpui::{
    App, AsyncApp, Bounds, Context, Hsla, Pixels, QuitMode, Render, Rgba, Window, WindowHandle,
    div, px,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};
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
    /// Whether the key-capture listener suspended the global keys last tick.
    pub capturing: bool,
    /// Value waiting for the next key pressed in the settings window.
    pub capture: Option<crate::ui::capture::CaptureTarget>,
    /// Combination detected during the capture, waiting for confirmation.
    pub capture_pending: Option<crate::core::KeyChord>,
    /// Radial wheel selection state (window lands in F3).
    pub overlay: OverlayState,
    /// Tray icon service.
    pub tray: Option<crate::platform::TrayService>,
    /// Language the tray icon was built with.
    pub tray_language: Language,
    /// Tray creation failed.
    pub tray_failed: bool,
    /// Quit requested.
    pub quitting: bool,
    /// Open settings window handle.
    pub settings_window: Option<WindowHandle<SettingsView>>,
    /// Last known settings bounds, restored on recreate.
    pub settings_bounds: Option<Bounds<Pixels>>,
    /// Open radial overlay window.
    pub overlay_window: Option<WindowHandle<OverlayView>>,
    /// Overlay window awaiting deferred removal after close.
    pub overlay_closing: Option<OverlayClosing>,
    /// Volume or brightness indicator currently on screen, if any.
    pub osd: Option<Osd>,
    /// Open volume indicator window.
    pub osd_window: Option<WindowHandle<OsdView>>,
}

/// Overlay window that was hidden but is not yet removed from gpui's map.
///
/// gpui dispatches active-status callbacks asynchronously, so the window must
/// stay registered until its deactivation callback has drained. Removing it
/// sooner makes that callback update a stale handle and log an error.
pub struct OverlayClosing {
    /// Handle kept alive until gpui has delivered the deactivation callback.
    pub handle: WindowHandle<OverlayView>,
    /// Earliest instant at which the window may be removed.
    pub remove_at: Instant,
}

/// Shared handle type.
pub type Shared = Rc<RefCell<ShellState>>;

impl ShellState {
    fn new(runtime: RuntimeHandle, cx: &mut App) -> Self {
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
            capturing: false,
            capture: None,
            capture_pending: None,
            overlay: OverlayState::default(),
            tray: None,
            tray_language: preferences.language,
            tray_failed: false,
            quitting: false,
            settings_window: None,
            settings_bounds: None,
            overlay_window: None,
            overlay_closing: None,
            osd: None,
            osd_window: None,
        }
    }
}

/// Root view of the settings window.
pub struct SettingsView {
    /// Shared shell state.
    pub shared: Shared,
}

impl SettingsView {
    fn sidebar(&self, cx: &mut Context<Self>, state: &ShellState) -> impl IntoElement {
        let t = style::tokens(state.preferences.theme_mode);
        let language = state.preferences.language;
        let shared = self.shared.clone();
        let weak = cx.weak_entity();

        let nav = SettingsTab::MAIN
            .iter()
            .map(|tab| nav_item(*tab, state.tab == *tab, t, language, &shared, &weak))
            .collect::<Vec<_>>();

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
                    .child(tr(language, TextKey::NavigationLabel)),
            )
            .children(nav)
            .child(div().flex_1())
            .child(
                div()
                    .mt_2()
                    .border_t_1()
                    .border_color(t.border)
                    .pt_2()
                    .child(nav_item(
                        SettingsTab::Settings,
                        state.tab == SettingsTab::Settings,
                        t,
                        language,
                        &shared,
                        &weak,
                    )),
            )
            .child(
                div()
                    .mt_3()
                    .text_size(px(10.5))
                    .text_color(t.muted)
                    .child(format!("v{VERSION} · GPL-3.0")),
            )
    }
}

/// One sidebar tab, used both in the main list and for the pinned Settings tab.
fn nav_item(
    tab: SettingsTab,
    selected: bool,
    t: style::Tokens,
    language: Language,
    shared: &Shared,
    weak: &gpui::WeakEntity<SettingsView>,
) -> impl IntoElement {
    let shared = shared.clone();
    let weak = weak.clone();
    div()
        .id(tab.label(language))
        .w_full()
        .flex()
        .items_center()
        .gap_2()
        .px_3()
        .py(px(7.0))
        .rounded(px(style::RADIUS_CONTROL))
        .when(selected, |div| div.bg(t.accent_soft))
        .when(selected, |div| div.text_color(style::ACCENT))
        .when(!selected, |div| div.text_color(t.muted))
        .hover(move |div| if selected { div } else { div.bg(t.card_hover) })
        .cursor_pointer()
        .child(
            gpui_icons::lucide(tab_icon(tab))
                .size(px(15.0))
                .text_color(if selected { style::ACCENT } else { t.muted }),
        )
        .child(tab.label(language))
        .on_click(move |_, _, cx| {
            shared.borrow_mut().tab = tab;
            if let Some(view) = weak.upgrade() {
                view.update(cx, |_, cx| cx.notify());
            }
        })
}

/// Drawer icon for one settings tab.
fn tab_icon(tab: SettingsTab) -> gpui_icons::LucideIcon {
    match tab {
        SettingsTab::General => gpui_icons::LucideIcon::LayoutDashboard,
        SettingsTab::Devices => gpui_icons::LucideIcon::SlidersHorizontal,
        SettingsTab::Actions => gpui_icons::LucideIcon::Waypoints,
        SettingsTab::Macros => gpui_icons::LucideIcon::ScrollText,
        SettingsTab::Settings => gpui_icons::LucideIcon::Settings,
    }
}

impl Render for SettingsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.shared.borrow();
        let t = style::tokens(state.preferences.theme_mode);
        let key_weak = cx.weak_entity();
        let sidebar = self.sidebar(cx, &state);
        let focus = state.page.focus_capture.clone();
        let key_shared = self.shared.clone();

        // The key listener is a sibling, not an ancestor: a focusable root
        // would take focus back on every click and starve text fields.
        let listener = div()
            .id("capture-listener")
            .absolute()
            .size(px(0.0))
            .track_focus(&focus)
            .on_key_down(move |event: &gpui::KeyDownEvent, _, cx| {
                if crate::ui::capture::handle_key(&key_shared, event)
                    && let Some(view) = key_weak.upgrade()
                {
                    view.update(cx, |_, cx| cx.notify());
                }
            });

        div()
            .id("settings-root")
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
            .child(listener)
    }
}

fn h(c: Rgba) -> Hsla {
    c.into()
}

fn mix_rgba(a: Rgba, b: Rgba, t: f32) -> Rgba {
    let t = t.clamp(0.0, 1.0);
    Rgba {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: a.a + (b.a - a.a) * t,
    }
}

/// Syncs gpui-kit's global theme with the application's palette so kit-backed
/// controls follow the app's dark/light mode and visual tokens.
fn sync_kit_theme(mode: ThemeMode, cx: &mut App) {
    let kit_mode = match mode {
        ThemeMode::Dark => gpui_kit::component::ThemeMode::Dark,
        ThemeMode::Light => gpui_kit::component::ThemeMode::Light,
    };
    gpui_kit::component::Theme::change(kit_mode, None, cx);

    let t = style::tokens(mode);
    let accent = h(style::ACCENT);
    let accent_soft = h(t.accent_soft);
    let bg = h(t.bg);
    let card = h(t.card);
    let card_hover = h(t.card_hover);
    let border = h(t.border);
    let text = h(t.text);
    let muted = h(t.muted);
    let success = h(style::SUCCESS);
    let warning = h(style::WARNING);
    let danger = h(style::DANGER);
    let on_accent = h(style::rgb(0xFFFFFF));

    gpui_kit::component::Theme::update(cx, |theme| {
        theme.radius = px(style::RADIUS_CONTROL);
        theme.radius_lg = px(style::RADIUS_CARD);

        let c = &mut theme.colors;
        c.background = bg;
        c.foreground = text;
        c.border = border;
        c.input = border;
        c.ring = accent;
        c.caret = accent;
        c.selection = accent_soft;
        c.accent = accent_soft;
        c.accent_foreground = text;

        c.muted = card_hover;
        c.muted_foreground = muted;

        c.popover = card;
        c.popover_foreground = text;

        c.primary = accent;
        c.primary_foreground = on_accent;
        c.primary_hover = h(style::rgb(0x3D79E6));
        c.primary_active = h(style::rgb(0x2F66CC));

        c.secondary = card_hover;
        c.secondary_foreground = text;
        c.secondary_hover = h(mix_rgba(t.card_hover, t.border, 0.35));
        c.secondary_active = h(mix_rgba(t.card_hover, t.border, 0.55));

        c.button = card;
        c.button_foreground = text;
        c.button_hover = card_hover;
        c.button_active = h(mix_rgba(t.card_hover, t.border, 0.25));
        c.button_primary = accent;
        c.button_primary_foreground = on_accent;
        c.button_primary_hover = c.primary_hover;
        c.button_primary_active = c.primary_active;
        c.button_secondary = card_hover;
        c.button_secondary_foreground = text;
        c.button_secondary_hover = c.secondary_hover;
        c.button_secondary_active = c.secondary_active;

        c.danger = danger;
        c.danger_foreground = on_accent;
        c.danger_hover = h(style::rgb(0xDC2626));
        c.danger_active = h(style::rgb(0xB91C1C));
        c.button_danger = danger;
        c.button_danger_foreground = on_accent;
        c.button_danger_hover = c.danger_hover;
        c.button_danger_active = c.danger_active;

        c.success = success;
        c.success_foreground = h(style::rgb(0x062016));
        c.success_hover = h(style::rgb(0x10B981));
        c.success_active = h(style::rgb(0x059669));
        c.button_success = success;
        c.button_success_foreground = c.success_foreground;
        c.button_success_hover = c.success_hover;
        c.button_success_active = c.success_active;

        c.warning = warning;
        c.warning_foreground = h(style::rgb(0x2A1600));
        c.warning_hover = h(style::rgb(0xD97706));
        c.warning_active = h(style::rgb(0xB45309));
        c.button_warning = warning;
        c.button_warning_foreground = c.warning_foreground;
        c.button_warning_hover = c.warning_hover;
        c.button_warning_active = c.warning_active;

        c.info = accent;
        c.info_foreground = on_accent;
        c.info_hover = c.primary_hover;
        c.info_active = c.primary_active;
        c.button_info = accent;
        c.button_info_foreground = on_accent;
        c.button_info_hover = c.primary_hover;
        c.button_info_active = c.primary_active;

        c.group_box = card;
        c.group_box_foreground = text;
        c.list = card;
        c.list_hover = card_hover;
        c.list_active = accent_soft;
        c.list_active_border = accent;
        c.list_head = card;
        c.list_even = card;
        c.table = card;
        c.table_hover = card_hover;
        c.table_active = accent_soft;
        c.table_active_border = accent;
        c.table_head = card;
        c.table_head_foreground = muted;
        c.table_even = card;
        c.table_row_border = border;
        c.tab_bar = h(t.sidebar);
        c.tab = h(t.sidebar);
        c.tab_active = card;
        c.tab_active_foreground = text;
        c.tab_foreground = muted;
        c.sidebar = h(t.sidebar);
        c.sidebar_foreground = text;
        c.sidebar_border = border;
        c.sidebar_accent = accent_soft;
        c.sidebar_accent_foreground = text;
        c.sidebar_primary = accent;
        c.sidebar_primary_foreground = on_accent;
        c.title_bar = h(t.sidebar);
        c.title_bar_border = border;
        c.status_bar = h(t.sidebar);
        c.status_bar_border = border;
        c.switch = border;
        c.switch_thumb = on_accent;
        c.slider_bar = accent;
        c.slider_thumb = on_accent;
        c.scrollbar = h(Rgba { a: 0.0, ..t.bg });
        c.scrollbar_thumb = h(mix_rgba(t.muted, t.bg, 0.35));
        c.scrollbar_thumb_hover = h(mix_rgba(t.muted, t.bg, 0.15));
        c.skeleton = card_hover;
        c.link = accent;
        c.link_hover = c.primary_hover;
        c.link_active = c.primary_active;
        c.overlay = h(Rgba {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.45,
        });
        c.window_border = border;
    });
}

/// Runs the gpui application. Called from `main` via `ui::run`.
pub fn run(runtime: RuntimeHandle) -> anyhow::Result<()> {
    gpui_platform::application()
        .with_assets(assets::WheelieAssets)
        .run(move |cx: &mut App| {
            cx.set_quit_mode(QuitMode::Explicit);
            gpui_kit::component::init(cx);

            if let Err(error) = assets::install_fonts(cx) {
                eprintln!("failed to load bundled fonts: {error}");
                return;
            }

            info!("gpui shell starting");
            let shared: Shared = Rc::new(RefCell::new(ShellState::new(runtime, cx)));
            crate::ui::controls::install_fields(cx, shared.clone());
            sync_kit_theme(shared.borrow().preferences.theme_mode, cx);
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
    let theme_before = shared.borrow().preferences.theme_mode;
    {
        let mut state = shared.borrow_mut();
        services::drain_events(&mut state);
        services::persist_preferences(&mut state);
        services::ensure_services(&mut state);
        services::sync_from_config(&mut state);
    }
    let theme_after = shared.borrow().preferences.theme_mode;
    if theme_before != theme_after {
        sync_kit_theme(theme_after, cx);
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
        }
        if hide {
            window::close_settings(cx, shared);
        }
        overlay_window::sync(cx, shared);
        osd_window::sync(cx, shared);
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
