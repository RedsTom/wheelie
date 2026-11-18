//! System tray icon with a small command menu.
//!
//! The icon must be created on the thread that runs the window event loop, so
//! the UI creates the service from its own `update` path. On Linux the tray
//! library needs a GTK main loop that eframe does not provide; creation is then
//! best-effort and the caller simply continues without a tray icon.

use crate::core::{APP_NAME, Language, TextKey, tr};
use crate::platform::error::PlatformError;
use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

/// Command chosen in the tray menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayCommand {
    /// Show and focus the settings window.
    Show,
    /// Hide the settings window.
    Hide,
    /// Suspend knob detection.
    Pause,
    /// Resume knob detection.
    Resume,
    /// Close Wheelie completely.
    Quit,
}

/// Tray icon and its menu. Drop to remove the icon.
pub struct TrayService {
    _tray: TrayIcon,
    entries: Vec<(MenuId, TrayCommand)>,
}

impl TrayService {
    /// Creates the tray icon with menu labels in `language`.
    pub fn new(language: Language) -> Result<Self, PlatformError> {
        let (rgba, width, height) = crate::assets::logo_rgba();
        let icon = Icon::from_rgba(rgba, width, height)
            .map_err(|error| PlatformError::Other(error.to_string()))?;

        let entries = [
            ("tray-show", TextKey::TrayShow, TrayCommand::Show),
            ("tray-hide", TextKey::TrayHide, TrayCommand::Hide),
            ("tray-pause", TextKey::TrayPause, TrayCommand::Pause),
            ("tray-resume", TextKey::TrayResume, TrayCommand::Resume),
            ("tray-quit", TextKey::TrayQuit, TrayCommand::Quit),
        ];

        let show = MenuItem::with_id("tray-show", tr(language, TextKey::TrayShow), true, None);
        let hide = MenuItem::with_id("tray-hide", tr(language, TextKey::TrayHide), true, None);
        let pause = MenuItem::with_id("tray-pause", tr(language, TextKey::TrayPause), true, None);
        let resume =
            MenuItem::with_id("tray-resume", tr(language, TextKey::TrayResume), true, None);
        let quit = MenuItem::with_id("tray-quit", tr(language, TextKey::TrayQuit), true, None);
        let first_separator = PredefinedMenuItem::separator();
        let second_separator = PredefinedMenuItem::separator();

        let menu = Menu::new();
        menu.append_items(&[
            &show,
            &hide,
            &first_separator,
            &pause,
            &resume,
            &second_separator,
            &quit,
        ])
        .map_err(|error| PlatformError::Other(error.to_string()))?;

        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip(APP_NAME)
            .with_icon(icon)
            .with_menu_on_left_click(true)
            .build()
            .map_err(|error| PlatformError::Other(error.to_string()))?;

        Ok(Self {
            _tray: tray,
            entries: entries
                .into_iter()
                .map(|(id, _, command)| (MenuId::new(id), command))
                .collect(),
        })
    }

    /// Drains pending menu selections.
    pub fn poll(&self) -> Vec<TrayCommand> {
        let mut commands = Vec::new();
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            if let Some((_, command)) = self.entries.iter().find(|(id, _)| id == &event.id) {
                commands.push(*command);
            }
        }
        commands
    }
}
