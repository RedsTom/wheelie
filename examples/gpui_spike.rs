//! F0 migration spike — proves the risky assumptions before porting `src/ui`:
//!
//! 1. A transparent, undecorated, centered `PopUp` window whose non-wheel
//!    pixels let the real desktop show through (zed#65402 watch).
//! 2. Rubik fonts loaded via `text_system().add_fonts` and SVG icons via
//!    `AssetSource` + the `svg()` element.
//! 3. Wheel sectors painted with `canvas` + `PathBuilder::arc_to`.
//! 4. tray-icon + global-hotkey created on the main thread and polled from a
//!    35 ms timer under the gpui event loop.
//! 5. Close-to-tray: closing the settings window keeps the app alive
//!    (`QuitMode::Explicit`) and the tray **Show** command recreates it.
//!
//! Run with: `cargo run --example gpui_spike`

use std::borrow::Cow;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use gpui::prelude::*;
use gpui::{
    App, AssetSource, AsyncApp, Bounds, Context, Path, PathBuilder, Pixels, Point, QuitMode,
    Render, SharedString, TitlebarOptions, Window, WindowBackgroundAppearance, WindowBounds,
    WindowHandle, WindowKind, WindowOptions, canvas, div, point, px, rgba, rgb, size, svg,
};
use gpui_platform::application;
use wheelie::core::{KeyChord, KeyCode, Language};
use wheelie::platform::{GlobalHotkeyService, HotkeySource, TrayCommand, TrayService};

// ---------------------------------------------------------------------------
// Assets: Rubik fonts + one Phosphor SVG to prove the AssetSource pipeline.
// ---------------------------------------------------------------------------

struct SpikeAssets;

impl AssetSource for SpikeAssets {
    fn load(&self, path: &str) -> anyhow::Result<Option<Cow<'static, [u8]>>> {
        Ok(match path {
            "icons/plus.svg" => Some(Cow::Borrowed(include_bytes!(
                "../assets/icons/plus.svg"
            ))),
            _ => None,
        })
    }

    fn list(&self, path: &str) -> anyhow::Result<Vec<SharedString>> {
        Ok(match path {
            "icons" => vec![SharedString::from("plus.svg")],
            _ => Vec::new(),
        })
    }
}

// ---------------------------------------------------------------------------
// Shared spike state — lives for the whole app, across window recreations.
// ---------------------------------------------------------------------------

struct SpikeState {
    ticks: usize,
    hotkey_events: usize,
    last_tray: &'static str,
    settings: Option<WindowHandle<SettingsView>>,
    overlay: Option<WindowHandle<OverlayView>>,
    settings_needs_open: bool,
    pending_close: Vec<WindowHandle<SettingsView>>,
    tray: Option<TrayService>,
    hotkeys: Option<GlobalHotkeyService>,
    quit: Arc<AtomicBool>,
}

type Shared = Rc<RefCell<SpikeState>>;

impl SpikeState {
    fn new() -> Self {
        Self {
            ticks: 0,
            hotkey_events: 0,
            last_tray: "-",
            settings: None,
            overlay: None,
            settings_needs_open: false,
            pending_close: Vec::new(),
            tray: None,
            hotkeys: None,
            quit: Arc::new(AtomicBool::new(false)),
        }
    }
}

// ---------------------------------------------------------------------------
// Overlay window: transparent 380×380 PopUp with wheel-like sectors.
// ---------------------------------------------------------------------------

fn polar(center: Point<Pixels>, r: f32, a: f32) -> Point<Pixels> {
    point(center.x + px(r * a.cos()), center.y + px(r * a.sin()))
}

/// Builds a filled annular sector between angles `a0` and `a1` (radians).
fn sector(center: Point<Pixels>, r0: f32, r1: f32, a0: f32, a1: f32) -> Path {
    let mut builder = PathBuilder::fill();
    builder.move_to(polar(center, r1, a0));
    builder.arc_to(
        point(px(r1), px(r1)),
        px(0.),
        (a1 - a0) > std::f32::consts::PI,
        true,
        polar(center, r1, a1),
    );
    builder.line_to(polar(center, r0, a1));
    builder.arc_to(
        point(px(r0), px(r0)),
        px(0.),
        false,
        false,
        polar(center, r0, a0),
    );
    builder.close();
    builder.build().unwrap()
}

struct OverlayView {
    shared: Shared,
}

impl Render for OverlayView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let ticks = self.shared.borrow().ticks;
        let gap = 0.06f32;
        let colors = [rgb(0x4B8BFF), rgb(0x3DDC97), rgb(0xFFB020)];
        div()
            .size_full()
            .relative()
            .child(
                canvas(
                    |_, _, _| (),
                    move |_bounds, _, window, _| {
                        let center = point(px(190.), px(190.));
                        for (i, color) in colors.iter().enumerate() {
                            let a0 = -std::f32::consts::FRAC_PI_2
                                + i as f32 * (std::f32::consts::TAU / 3.0)
                                + gap;
                            let a1 = -std::f32::consts::FRAC_PI_2
                                + (i + 1) as f32 * (std::f32::consts::TAU / 3.0)
                                - gap;
                            window.paint_path(sector(center, 70., 160., a0, a1), *color);
                        }
                        // Center hub (two half-discs: a single full arc is degenerate).
                        let hub = rgba(0x141414E0);
                        window.paint_path(sector(center, 0., 58., 0., std::f32::consts::PI), hub);
                        window.paint_path(
                            sector(center, 0., 58., std::f32::consts::PI, std::f32::consts::TAU),
                            hub,
                        );
                    },
                )
                .absolute()
                .inset_0(),
            )
            .child(
                svg()
                    .path("icons/plus.svg")
                    .size(px(36.))
                    .text_color(rgb(0x4B8BFF))
                    .absolute()
                    .top(px(172.))
                    .left(px(172.)),
            )
            .child(
                div()
                    .absolute()
                    .bottom(px(8.))
                    .left(px(110.))
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .bg(rgb(0x141414).alpha(0.75))
                    .font_family("Rubik")
                    .text_size(px(11.))
                    .text_color(rgb(0xE8E8E8))
                    .child(format!("overlay tick {ticks}")),
            )
    }
}

// ---------------------------------------------------------------------------
// Settings window: normal decorated window with the spike counters.
// ---------------------------------------------------------------------------

struct SettingsView {
    shared: Shared,
}

impl Render for SettingsView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let (ticks, hotkeys, last_tray, tray_ok, hotkeys_ok) = {
            let state = self.shared.borrow();
            (
                state.ticks,
                state.hotkey_events,
                state.last_tray,
                state.tray.is_some(),
                state.hotkeys.is_some(),
            )
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap_3()
            .p_6()
            .bg(rgb(0x1E1E1E))
            .font_family("Rubik")
            .text_color(rgb(0xE8E8E8))
            .text_size(px(14.))
            .child("gpui spike — F0 probes")
            .child(div().child(format!("timer ticks (35 ms): {ticks}")))
            .child(div().child(format!("hotkey events (F14): {hotkeys}")))
            .child(div().child(format!("last tray command: {last_tray}")))
            .child(div().child(format!(
                "tray created: {tray_ok} · hotkeys created: {hotkeys_ok}"
            )))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .items_center()
                    .child(
                        svg()
                            .path("icons/plus.svg")
                            .size(px(20.))
                            .text_color(rgb(0x4B8BFF)),
                    )
                    .child("Rubik + SVG OK — close this window: app must survive in the tray"),
            )
    }
}

// ---------------------------------------------------------------------------
// Windows accent policy override — the border probe.
//
// gpui sets ACCENT_ENABLE_TRANSPARENTGRADIENT (state 2) for transparent
// windows, which draws an accent-tinted frame (zed#65402). With
// WS_EX_NOREDIRECTIONBITMAP the DirectComposition visual carries alpha by
// itself, so ACCENT_DISABLED (state 0) may keep transparency and lose the
// border. `SPIKE_ACCENT` selects the state: "0" (default), "2" (gpui
// baseline), or "none" (skip the call).
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
fn apply_accent_state(window: &mut Window) {
    use std::ffi::c_void;

    #[repr(C)]
    struct AccentPolicy {
        accent_state: u32,
        accent_flags: u32,
        gradient_color: u32,
        animation_id: u32,
    }

    #[repr(C)]
    struct WindowCompositionAttributeData {
        attribute: u32,
        data: *const c_void,
        size: usize,
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetModuleHandleA(name: *const std::ffi::c_char) -> *mut c_void;
        fn GetProcAddress(module: *mut c_void, name: *const std::ffi::c_char) -> *mut c_void;
    }

    #[link(name = "dwmapi")]
    unsafe extern "system" {
        fn DwmSetWindowAttribute(
            hwnd: *mut c_void,
            attribute: u32,
            value: *const c_void,
            size: u32,
        ) -> i32;
    }

    type SetWindowCompositionAttributeFn =
        unsafe extern "system" fn(*mut c_void, *mut WindowCompositionAttributeData) -> i32;

    // SPIKE_ACCENT=0|2|4 selects the accent state; 4 (acrylic) must blur
    // visibly, proving this call actually lands after gpui's own
    // set_background_appearance. Unset → leave gpui's default (state 2) alone:
    // the accent policy is not the source of the window border, so the
    // override stays opt-in until the border issue is tackled.
    let state = match std::env::var("SPIKE_ACCENT").as_deref() {
        Ok("0") => 0,
        Ok("2") => 2,
        Ok("4") => 4,
        _ => return,
    };

    let raw = match raw_window_handle::HasWindowHandle::window_handle(window)
        .map(|handle| handle.as_raw())
    {
        Ok(raw_window_handle::RawWindowHandle::Win32(win32)) => win32.hwnd.get() as *mut c_void,
        _ => return,
    };

    // SPIKE_CORNER=donotround → square corners (probe: is the line the Win11
    // rounded-window border?).
    if std::env::var("SPIKE_CORNER").as_deref() == Ok("donotround") {
        let preference: u32 = 1; // DWMWCP_DONOTROUND
        unsafe {
            let _ = DwmSetWindowAttribute(
                raw,
                33, // DWMWA_WINDOW_CORNER_PREFERENCE
                &preference as *const u32 as *const c_void,
                std::mem::size_of::<u32>() as u32,
            );
        }
    }

    unsafe {
        let module = GetModuleHandleA(c"user32.dll".as_ptr());
        if module.is_null() {
            return;
        }
        let symbol = GetProcAddress(module, c"SetWindowCompositionAttribute".as_ptr());
        if symbol.is_null() {
            return;
        }
        let set: SetWindowCompositionAttributeFn = std::mem::transmute(symbol);
        let policy = AccentPolicy {
            accent_state: state,
            accent_flags: if state == 4 { 0 } else { 2 },
            // Acrylic needs a non-zero alpha or DWM ignores the policy.
            gradient_color: if state == 4 { 0x01000000 } else { 0 },
            animation_id: 0,
        };
        let mut data = WindowCompositionAttributeData {
            attribute: 19, // ACCENT_POLICY
            data: &policy as *const AccentPolicy as *const c_void,
            size: std::mem::size_of::<AccentPolicy>(),
        };
        let _ = set(raw, &mut data);
    }
}

#[cfg(not(target_os = "windows"))]
fn apply_accent_state(_window: &mut Window) {}

// ---------------------------------------------------------------------------
// Window management (close-to-tray via recreate).
// ---------------------------------------------------------------------------

fn open_overlay(cx: &mut App, shared: &Shared) {
    if shared.borrow().overlay.is_some() {
        return;
    }
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
            None,
            size(px(380.), px(380.)),
            cx,
        ))),
        titlebar: Some(TitlebarOptions {
            title: Some("Wheelie overlay spike".into()),
            appears_transparent: true,
            traffic_light_position: None,
        }),
        kind: WindowKind::PopUp,
        is_movable: false,
        is_resizable: false,
        is_minimizable: false,
        window_background: WindowBackgroundAppearance::Transparent,
        app_id: Some("com.redstom.wheelie.spike".into()),
        ..Default::default()
    };
    let handle = cx
        .open_window(options, |window, cx| {
            // Applied after gpui finishes applying WindowOptions (which would
            // otherwise overwrite the accent policy with state 2).
            window.defer(cx, |window, _| apply_accent_state(window));
            cx.new(|cx| OverlayView {
                shared: shared.clone(),
            })
        })
        .expect("open overlay window");
    shared.borrow_mut().overlay = Some(handle);
}

fn open_settings(cx: &mut App, shared: &Shared) {
    if shared.borrow().settings.is_some() {
        return;
    }
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
            None,
            size(px(560.), px(320.)),
            cx,
        ))),
        titlebar: Some(TitlebarOptions {
            title: Some("Wheelie spike".into()),
            ..Default::default()
        }),
        app_id: Some("com.redstom.wheelie.spike".into()),
        ..Default::default()
    };
    let handle = cx
        .open_window(options, |_, cx| {
            cx.new(|cx| SettingsView {
                shared: shared.clone(),
            })
        })
        .expect("open settings window");
    shared.borrow_mut().settings = Some(handle);
}

/// Drops stale handles of windows the user closed (X button).
fn reap_closed_windows(cx: &mut App, shared: &Shared) {
    let mut state = shared.borrow_mut();
    if let Some(handle) = &state.settings
        && handle.update(cx, |_, _, _| ()).is_err()
    {
        state.settings = None;
    }
    if let Some(handle) = &state.overlay
        && handle.update(cx, |_, _, _| ()).is_err()
    {
        state.overlay = None;
    }
}

fn notify_windows(cx: &mut App, shared: &Shared) {
    let state = shared.borrow();
    if let Some(handle) = &state.settings {
        let _ = handle.update(cx, |_, _, cx| cx.notify());
    }
    if let Some(handle) = &state.overlay {
        let _ = handle.update(cx, |_, _, cx| cx.notify());
    }
}

// ---------------------------------------------------------------------------
// 35 ms poll tick — the future eframe `update()` body.
// ---------------------------------------------------------------------------

fn poll_tick(cx: &mut App, shared: &Shared) {
    let mut open_settings_after = false;
    let mut close_settings = Vec::new();
    let mut quit_now = false;

    {
        let mut state = shared.borrow_mut();
        state.ticks += 1;

        if let Some(tray) = &state.tray {
            for command in tray.poll() {
                state.last_tray = match command {
                    TrayCommand::Show => "Show",
                    TrayCommand::Hide => "Hide",
                    TrayCommand::Pause => "Pause",
                    TrayCommand::Resume => "Resume",
                    TrayCommand::Quit => "Quit",
                };
                match command {
                    TrayCommand::Show => state.settings_needs_open = true,
                    TrayCommand::Hide => {
                        if let Some(handle) = state.settings.take() {
                            state.pending_close.push(handle);
                        }
                    }
                    TrayCommand::Quit => state.quit.store(true, Ordering::SeqCst),
                    TrayCommand::Pause | TrayCommand::Resume => {}
                }
            }
        }
        if let Some(hotkeys) = &mut state.hotkeys {
            state.hotkey_events += hotkeys.poll().len();
        }

        open_settings_after = state.settings_needs_open;
        state.settings_needs_open = false;
        close_settings.append(&mut state.pending_close);
        quit_now = state.quit.load(Ordering::SeqCst);
    }

    for handle in close_settings {
        let _ = handle.update(cx, |_, window, _| window.remove_window());
    }
    if open_settings_after {
        open_settings(cx, shared);
    }

    reap_closed_windows(cx, shared);
    notify_windows(cx, shared);

    if quit_now {
        cx.quit();
    }
}

// ---------------------------------------------------------------------------
// Entry point.
// ---------------------------------------------------------------------------

fn run_spike() {
    application()
        .with_assets(SpikeAssets)
        .run(move |cx: &mut App| {
            cx.set_quit_mode(QuitMode::Explicit);

            let _ = cx.text_system().add_fonts(vec![
            Cow::Borrowed(include_bytes!("../assets/fonts/Rubik-Regular.ttf").as_slice()),
            Cow::Borrowed(include_bytes!("../assets/fonts/Rubik-SemiBold.ttf").as_slice()),
        ]);

            let shared: Shared = Rc::new(RefCell::new(SpikeState::new()));

            // Tray + hotkeys: created here, on the main thread (AGENTS.md rule).
            {
                let mut state = shared.borrow_mut();
                let built = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    TrayService::new(Language::Fr)
                }));
                state.tray = match built {
                    Ok(Ok(tray)) => Some(tray),
                    Ok(Err(error)) => {
                        eprintln!("spike: tray creation failed: {error}");
                        None
                    }
                    Err(_) => {
                        eprintln!("spike: tray creation panicked");
                        None
                    }
                };
                match GlobalHotkeyService::new() {
                    Ok(mut hotkeys) => match hotkeys.apply(&[KeyChord::new(KeyCode::F14)]) {
                        Ok(()) => state.hotkeys = Some(hotkeys),
                        Err(error) => eprintln!("spike: hotkey registration failed: {error}"),
                    },
                    Err(error) => eprintln!("spike: hotkey creation failed: {error}"),
                }
            }

            open_overlay(cx, &shared);
            open_settings(cx, &shared);

            let tick_shared = shared.clone();
            cx.spawn(async move |cx: &mut AsyncApp| loop {
                cx.background_executor()
                    .timer(Duration::from_millis(35))
                    .await;
                let quit = cx.update(|app| {
                    poll_tick(app, &tick_shared);
                    tick_shared.borrow().quit.load(Ordering::SeqCst)
                });
                if quit {
                    break;
                }
            })
            .detach();
        });
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    run_spike();
}
