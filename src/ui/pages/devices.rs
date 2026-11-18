//! Devices page (gpui port in progress; blank until ported).

use crate::ui::shell::{SettingsView, Shared, ShellState};
use gpui::prelude::*;
use gpui::{Context, Window, div};

pub(crate) fn render(
    _state: &ShellState,
    _shared: &Shared,
    _window: &Window,
    _cx: &mut Context<SettingsView>,
) -> impl IntoElement {
    div()
}
