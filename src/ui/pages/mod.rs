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

/// Ephemeral page UI state: menus, text buffers and focus handles.
pub struct PageState {
    /// Id of the open dropdown menu, if any. `String` so rows can build
    /// dynamic ids like `step-3-kind`.
    pub open_menu: Option<String>,
    /// Focus handle for the action-set name field.
    pub focus_set_name: FocusHandle,
    /// Buffer for the action-set name field.
    pub set_name: String,
    /// Select-all-on-first-input flag for the set name field.
    pub set_name_fresh: bool,
    /// Focus handle for the macro name field.
    pub focus_macro_name: FocusHandle,
    /// Buffer for the macro name field.
    pub macro_name: String,
    /// Select-all-on-first-input flag for the macro name field.
    pub macro_name_fresh: bool,
    /// Focus handle for the macro text-step field.
    pub focus_macro_text: FocusHandle,
    /// Buffer for the macro text-step field.
    pub macro_text: String,
    /// Select-all-on-first-input flag for the macro text-step field.
    pub macro_text_fresh: bool,
    /// Focus handle for a numeric step field (delay/amount).
    pub focus_step_value: FocusHandle,
    /// Buffer for a numeric step field.
    pub step_value: String,
    /// Select-all-on-first-input flag for the numeric step field.
    pub step_value_fresh: bool,
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
    /// Focus handle for the launch-path field (actions page).
    pub focus_path: FocusHandle,
    /// Buffer for the launch-path field.
    pub path_buffer: String,
    /// Select-all-on-first-input flag for the launch-path field.
    pub path_buffer_fresh: bool,
    /// Focus of the settings root while a key capture is pending.
    pub focus_capture: FocusHandle,
}

impl PageState {
    pub(crate) fn new(cx: &App) -> Self {
        Self {
            open_menu: None,
            focus_set_name: cx.focus_handle(),
            set_name: String::new(),
            set_name_fresh: true,
            focus_macro_name: cx.focus_handle(),
            macro_name: String::new(),
            macro_name_fresh: true,
            focus_macro_text: cx.focus_handle(),
            macro_text: String::new(),
            macro_text_fresh: true,
            focus_step_value: cx.focus_handle(),
            step_value: String::new(),
            step_value_fresh: true,
            selected_set: None,
            hovered_set: None,
            hovered_step: None,
            selected_macro: None,
            selected_step: None,
            preview_hovered: None,
            focus_path: cx.focus_handle(),
            path_buffer: String::new(),
            path_buffer_fresh: true,
            focus_capture: cx.focus_handle(),
        }
    }
}

/// Renders the page selected by the shell.
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

/// Translated label and color for a runtime state.
pub fn state_label(language: Language, state: RuntimeState) -> (&'static str, Rgba) {
    match state {
        RuntimeState::Starting => (tr(language, TextKey::StateStarting), style::WARNING),
        RuntimeState::Running => (tr(language, TextKey::StateRunning), style::SUCCESS),
        RuntimeState::Paused => (tr(language, TextKey::StatePaused), style::WARNING),
        RuntimeState::Stopped => (tr(language, TextKey::StateStopped), style::DANGER),
    }
}
