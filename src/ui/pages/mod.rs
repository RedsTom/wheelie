//! gpui settings pages.
//!
//! Each page reads the shared [`ShellState`] and renders from it; mutations
//! happen in event closures that borrow the shared state and notify the
//! settings view.

pub(crate) mod actions;
pub(crate) mod devices;
mod general;
pub(crate) mod macros;
pub(crate) mod onboarding;
mod settings;

use crate::app::RuntimeState;
use crate::core::{CaptureMode, Language, PlatformInfo, TextKey, tr};
use crate::ui::shell::{SettingsView, Shared, ShellState};
use crate::ui::style::{self, Tokens};
use gpui::prelude::*;
use gpui::{AnyElement, App, Context, FocusHandle, Rgba, Window, div, px};

/// Ephemeral page UI state: open menus, hovered rows and current selections.
///
/// Text input state lives in gpui-kit's `InputState` (see `ui::controls`), so
/// this struct no longer carries buffers or one focus handle per field kind.
pub struct PageState {
    /// Id of the open dropdown menu, if any. `String` so rows can build
    /// dynamic ids like `step-3-kind`.
    pub open_menu: Option<String>,
    /// Control whose key assignment conflicts with another control.
    pub binding_conflict: Option<crate::core::Control>,
    /// Currently edited action set (by id).
    pub selected_set: Option<String>,
    /// Hovered action-set row in the list; drives the reorder buttons.
    pub hovered_set: Option<String>,
    /// Hovered macro step row; drives the reorder buttons.
    pub hovered_step: Option<usize>,
    /// Currently edited macro (by id).
    pub selected_macro: Option<String>,
    /// Currently edited macro step (by index).
    pub selected_step: Option<usize>,
    /// Hovered sector in the actions wheel preview.
    pub preview_hovered: Option<usize>,
    /// Focus of the settings root while a key capture is pending.
    pub focus_capture: FocusHandle,
}

impl PageState {
    pub(crate) fn new(cx: &App) -> Self {
        Self {
            open_menu: None,
            binding_conflict: None,
            selected_set: None,
            hovered_set: None,
            hovered_step: None,
            selected_macro: None,
            selected_step: None,
            preview_hovered: None,
            focus_capture: cx.focus_handle(),
        }
    }
}

/// Renders the page selected by the shell.
///
/// The tab descriptor table in [`crate::ui::nav`] is the single source of
/// truth; this dispatch only maps an id to its renderer.
pub fn render(
    state: &ShellState,
    shared: &Shared,
    window: &mut Window,
    cx: &mut Context<SettingsView>,
) -> AnyElement {
    use crate::ui::nav::SettingsTab;
    match state.tab {
        SettingsTab::General => general::render(state, shared, window, cx).into_any_element(),
        SettingsTab::Settings => settings::render(state, shared, window, cx).into_any_element(),
        SettingsTab::Devices => devices::render(state, shared, window, cx).into_any_element(),
        SettingsTab::Macros => macros::render(state, shared, window, cx).into_any_element(),
        SettingsTab::Actions => actions::render(state, shared, window, cx).into_any_element(),
    }
}

/// Stable accessibility id for a sidebar tab, independent of the active
/// language so switching language does not recreate the focus node.
pub fn tab_element_id(tab: crate::ui::nav::SettingsTab) -> String {
    format!("settings-tab-{tab:?}")
}

/// Hairline separator inside a card.
pub fn separator(t: Tokens) -> impl IntoElement {
    gpui_kit::component::separator::Separator::horizontal().color(gpui::Hsla::from(t.border))
}

/// Muted value row: label left, value right.
pub fn info_row(t: Tokens, label: &str, value: &str) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
        .pt(px(4.0))
        .child(
            div()
                .text_size(px(12.5))
                .text_color(t.muted)
                .child(label.to_owned()),
        )
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .text_size(px(12.5))
                .text_color(t.text)
                .text_right()
                .child(value.to_owned()),
        )
}

/// Inline warning line used by pages that validate user input.
pub fn warning(text: &str) -> impl IntoElement {
    div()
        .text_size(px(12.5))
        .text_color(style::WARNING)
        .child(text.to_owned())
}

/// Platform permission or limitation notice relevant to knob keys.
pub fn platform_notice_text(language: Language, platform: PlatformInfo) -> Option<&'static str> {
    let key = match platform.capture {
        CaptureMode::PermissionedHotkeys => Some(TextKey::MacPermissionNotice),
        CaptureMode::X11BestEffort => Some(TextKey::LinuxCaptureNotice),
        CaptureMode::ExclusiveHotkeys => Some(TextKey::WindowsElevatedNotice),
        CaptureMode::Unsupported => None,
    };
    key.map(|key| tr(language, key))
}

/// Translated label and opaque, AA-contrasting ink for a runtime state.
pub fn state_label(
    language: Language,
    state: RuntimeState,
    mode: crate::core::ThemeMode,
) -> (&'static str, Rgba) {
    let color = match state {
        RuntimeState::Starting | RuntimeState::Paused => style::WARNING,
        RuntimeState::Running => style::SUCCESS,
        RuntimeState::Stopped => style::DANGER,
    };
    let label = match state {
        RuntimeState::Starting => tr(language, TextKey::StateStarting),
        RuntimeState::Running => tr(language, TextKey::StateRunning),
        RuntimeState::Paused => tr(language, TextKey::StatePaused),
        RuntimeState::Stopped => tr(language, TextKey::StateStopped),
    };
    (label, style::on_surface(color, mode))
}
