//! System tray icon with a small command menu.
//!
//! The icon must be created on the thread that runs the window event loop, so
//! the UI creates the service from its own `update` path. On Linux the tray
//! library needs a GTK main loop that the shell does not provide; creation is
//! then best-effort and the caller simply continues without a tray icon.

use crate::core::{APP_NAME, Language, TextKey, tr};
use crate::platform::error::PlatformError;
use tray_icon::menu::{IsMenuItem, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TrayMenuState {
    language: Language,
    window_visible: bool,
    paused: bool,
}

/// Tray icon and its menu. Drop to remove the icon.
pub struct TrayService {
    tray: TrayIcon,
    entries: Vec<(MenuId, TrayCommand)>,
    state: TrayMenuState,
}

impl TrayService {
    /// Creates the tray icon. The first menu is set by `set_state` on the next
    /// UI tick, once the window and runtime state are known.
    pub fn new(language: Language) -> Result<Self, PlatformError> {
        let (rgba, width, height) = crate::assets::logo_rgba();
        let icon = Icon::from_rgba(rgba, width, height)
            .map_err(|error| PlatformError::Other(error.to_string()))?;

        let state = TrayMenuState {
            language,
            window_visible: true,
            paused: false,
        };
        let (menu, entries) = build_menu(state)?;
        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip(APP_NAME)
            .with_icon(icon)
            .with_menu_on_left_click(true)
            .build()
            .map_err(|error| PlatformError::Other(error.to_string()))?;

        Ok(Self {
            tray,
            entries,
            state,
        })
    }

    /// Rebuilds the menu with only the actions that make sense right now.
    pub fn set_state(
        &mut self,
        language: Language,
        window_visible: bool,
        paused: bool,
    ) -> Result<(), PlatformError> {
        let next = TrayMenuState {
            language,
            window_visible,
            paused,
        };
        if next == self.state {
            return Ok(());
        }

        let (menu, entries) = build_menu(next)?;
        self.tray.set_menu(Some(Box::new(menu)));
        self.entries = entries;
        self.state = next;
        Ok(())
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

/// The window and pause pairs are mutually exclusive; quitting is always
/// available.
fn tray_commands(window_visible: bool, paused: bool) -> Vec<TrayCommand> {
    vec![
        if window_visible {
            TrayCommand::Hide
        } else {
            TrayCommand::Show
        },
        if paused {
            TrayCommand::Resume
        } else {
            TrayCommand::Pause
        },
        TrayCommand::Quit,
    ]
}

fn build_menu(state: TrayMenuState) -> Result<(Menu, Vec<(MenuId, TrayCommand)>), PlatformError> {
    let mut items: Vec<Box<dyn IsMenuItem>> = Vec::new();
    let mut entries = Vec::new();

    for (index, command) in tray_commands(state.window_visible, state.paused)
        .into_iter()
        .enumerate()
    {
        if index > 0 {
            items.push(Box::new(PredefinedMenuItem::separator()));
        }

        let (id, label) = match command {
            TrayCommand::Show => ("tray-show", TextKey::TrayShow),
            TrayCommand::Hide => ("tray-hide", TextKey::TrayHide),
            TrayCommand::Pause => ("tray-pause", TextKey::TrayPause),
            TrayCommand::Resume => ("tray-resume", TextKey::TrayResume),
            TrayCommand::Quit => ("tray-quit", TextKey::TrayQuit),
        };
        items.push(Box::new(MenuItem::with_id(
            id,
            tr(state.language, label),
            true,
            None,
        )));
        entries.push((MenuId::new(id), command));
    }

    let menu = Menu::new();
    let refs: Vec<&dyn IsMenuItem> = items.iter().map(|item| item.as_ref()).collect();
    menu.append_items(&refs)
        .map_err(|error| PlatformError::Other(error.to_string()))?;
    Ok((menu, entries))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_shows_hide_while_the_window_is_open() {
        assert_eq!(
            tray_commands(true, false),
            vec![TrayCommand::Hide, TrayCommand::Pause, TrayCommand::Quit]
        );
        assert_eq!(
            tray_commands(false, false),
            vec![TrayCommand::Show, TrayCommand::Pause, TrayCommand::Quit]
        );
    }

    #[test]
    fn menu_shows_resume_only_while_paused() {
        assert_eq!(
            tray_commands(true, true),
            vec![TrayCommand::Hide, TrayCommand::Resume, TrayCommand::Quit]
        );
        assert_eq!(
            tray_commands(false, true),
            vec![TrayCommand::Show, TrayCommand::Resume, TrayCommand::Quit]
        );
    }
}
