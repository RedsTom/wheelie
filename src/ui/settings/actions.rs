//! Actions page: choose what each knob gesture does.

use crate::core::action::{MediaCommand, MouseButton, ScrollAxis, VolumeDirection};
use crate::core::profile::Actions;
use crate::core::{Action, Config, Control, KeyChord, KeyCode, Language, TextKey, tr};
use crate::ui::settings::{PageContext, active_profile_index, warning};
use crate::ui::{theme, widgets};
use eframe::egui;
use std::path::PathBuf;

pub fn show(ui: &mut egui::Ui, context: &mut PageContext<'_>) {
    let language = context.language;
    widgets::page_header(
        ui,
        tr(language, TextKey::TabActions),
        tr(language, TextKey::ActionsSubtitle),
    );

    let config = context.runtime.config();
    let Some(index) = active_profile_index(&config) else {
        return;
    };

    let mut actions = config.profiles[index].actions.clone();
    let mut changed = false;

    for control in [Control::Left, Control::Press, Control::Right] {
        widgets::card(ui, |ui| {
            if show_control(ui, language, &config, control, &mut actions) {
                changed = true;
            }
        });
        ui.add_space(10.0);
    }

    if changed {
        let result = context
            .runtime
            .update_config(|config| config.profiles[index].actions = actions.clone());
        if let Err(error) = result {
            warning(ui, &error.to_string());
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    None,
    Volume,
    Scroll,
    Macro,
    Shortcut,
    Mouse,
    Media,
    Launch,
}

impl Kind {
    const ALL: [Self; 8] = [
        Self::None,
        Self::Volume,
        Self::Scroll,
        Self::Macro,
        Self::Shortcut,
        Self::Mouse,
        Self::Media,
        Self::Launch,
    ];

    fn of(action: &Action) -> Self {
        match action {
            Action::None => Self::None,
            Action::Volume { .. } => Self::Volume,
            Action::Scroll { .. } => Self::Scroll,
            Action::Macro { .. } => Self::Macro,
            Action::Shortcut { .. } => Self::Shortcut,
            Action::MouseClick { .. } => Self::Mouse,
            Action::Media { .. } => Self::Media,
            Action::Launch { .. } => Self::Launch,
        }
    }

    fn label(self, language: Language) -> &'static str {
        match self {
            Self::None => tr(language, TextKey::ActionNone),
            Self::Volume => tr(language, TextKey::ActionKindVolume),
            Self::Scroll => tr(language, TextKey::ActionKindScroll),
            Self::Macro => tr(language, TextKey::ActionKindMacro),
            Self::Shortcut => tr(language, TextKey::ActionKindShortcut),
            Self::Mouse => tr(language, TextKey::ActionKindMouse),
            Self::Media => tr(language, TextKey::ActionKindMedia),
            Self::Launch => tr(language, TextKey::ActionKindLaunch),
        }
    }

    fn default_action(self, config: &Config) -> Action {
        match self {
            Self::None => Action::None,
            Self::Volume => Action::Volume {
                direction: VolumeDirection::Up,
                steps: 1,
            },
            Self::Scroll => Action::Scroll {
                axis: ScrollAxis::Vertical,
                amount: -3,
            },
            Self::Macro => match config.macros.first() {
                Some(definition) => Action::Macro {
                    name: definition.name.clone(),
                },
                None => Action::None,
            },
            Self::Shortcut => Action::Shortcut {
                chord: KeyChord::new(KeyCode::F13),
            },
            Self::Mouse => Action::MouseClick {
                button: MouseButton::Left,
            },
            Self::Media => Action::Media {
                command: MediaCommand::PlayPause,
            },
            Self::Launch => Action::Launch {
                path: PathBuf::from("."),
            },
        }
    }
}

fn show_control(
    ui: &mut egui::Ui,
    language: Language,
    config: &Config,
    control: Control,
    actions: &mut Actions,
) -> bool {
    let mut changed = false;
    let control_key = match control {
        Control::Left => "left",
        Control::Right => "right",
        Control::Press => "press",
    };

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(control_label(language, control))
                .font(theme::heading_font(14.0))
                .color(theme::card_text(ui)),
        );

        let mut kind = Kind::of(actions.for_control(control));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            egui::ComboBox::from_id_salt(("action-kind", control_key))
                .selected_text(kind.label(language))
                .width(180.0)
                .show_ui(ui, |ui| {
                    for option in Kind::ALL {
                        ui.selectable_value(&mut kind, option, option.label(language));
                    }
                });
        });
        if kind != Kind::of(actions.for_control(control)) {
            let action = kind.default_action(config);
            match control {
                Control::Left => actions.left = action,
                Control::Right => actions.right = action,
                Control::Press => actions.press = action,
            }
            changed = true;
        }
    });

    ui.add_space(8.0);

    let action = match control {
        Control::Left => &mut actions.left,
        Control::Right => &mut actions.right,
        Control::Press => &mut actions.press,
    };
    changed |= parameters(ui, language, config, control_key, action);
    changed
}

fn parameters(
    ui: &mut egui::Ui,
    language: Language,
    config: &Config,
    key: &str,
    action: &mut Action,
) -> bool {
    let mut changed = false;

    match action {
        Action::None => {
            ui.label(
                egui::RichText::new(tr(language, TextKey::ActionNone))
                    .size(12.5)
                    .color(theme::muted(ui)),
            );
        }
        Action::Volume { direction, steps } => {
            ui.horizontal_wrapped(|ui| {
                muted_label(ui, language, TextKey::ActionDirection);
                egui::ComboBox::from_id_salt(("volume-direction", key))
                    .selected_text(direction_label(language, *direction))
                    .width(130.0)
                    .show_ui(ui, |ui| {
                        for option in [VolumeDirection::Up, VolumeDirection::Down] {
                            if ui
                                .selectable_value(
                                    direction,
                                    option,
                                    direction_label(language, option),
                                )
                                .changed()
                            {
                                changed = true;
                            }
                        }
                    });
                ui.add_space(10.0);
                muted_label(ui, language, TextKey::ActionSteps);
                changed |= ui.add(egui::DragValue::new(steps).range(1..=10)).changed();
            });
        }
        Action::Scroll { axis, amount } => {
            ui.horizontal_wrapped(|ui| {
                muted_label(ui, language, TextKey::ActionAxis);
                egui::ComboBox::from_id_salt(("scroll-axis", key))
                    .selected_text(axis_label(language, *axis))
                    .width(130.0)
                    .show_ui(ui, |ui| {
                        for option in [ScrollAxis::Vertical, ScrollAxis::Horizontal] {
                            if ui
                                .selectable_value(axis, option, axis_label(language, option))
                                .changed()
                            {
                                changed = true;
                            }
                        }
                    });
                ui.add_space(10.0);
                muted_label(ui, language, TextKey::ActionAmount);
                changed |= ui
                    .add(egui::DragValue::new(amount).range(-100..=100))
                    .changed();
            });
        }
        Action::Macro { name } => {
            ui.horizontal_wrapped(|ui| {
                muted_label(ui, language, TextKey::ActionMacroName);
                egui::ComboBox::from_id_salt(("action-macro", key))
                    .selected_text(if name.is_empty() {
                        "—".to_owned()
                    } else {
                        name.clone()
                    })
                    .width(200.0)
                    .show_ui(ui, |ui| {
                        for definition in &config.macros {
                            if ui
                                .selectable_value(name, definition.name.clone(), &definition.name)
                                .changed()
                            {
                                changed = true;
                            }
                        }
                    });
            });
        }
        Action::Shortcut { chord } => {
            ui.horizontal_wrapped(|ui| {
                muted_label(ui, language, TextKey::ActionChord);
                changed |= widgets::chord_picker(ui, ("action-chord", key), language, chord);
            });
        }
        Action::MouseClick { button } => {
            ui.horizontal_wrapped(|ui| {
                muted_label(ui, language, TextKey::ActionButton);
                egui::ComboBox::from_id_salt(("action-mouse", key))
                    .selected_text(button.label(language))
                    .width(160.0)
                    .show_ui(ui, |ui| {
                        for option in [MouseButton::Left, MouseButton::Right, MouseButton::Middle] {
                            if ui
                                .selectable_value(button, option, option.label(language))
                                .changed()
                            {
                                changed = true;
                            }
                        }
                    });
            });
        }
        Action::Media { command } => {
            ui.horizontal_wrapped(|ui| {
                muted_label(ui, language, TextKey::ActionCommand);
                egui::ComboBox::from_id_salt(("action-media", key))
                    .selected_text(media_label(language, *command))
                    .width(180.0)
                    .show_ui(ui, |ui| {
                        for option in [
                            MediaCommand::PlayPause,
                            MediaCommand::Next,
                            MediaCommand::Previous,
                            MediaCommand::Stop,
                        ] {
                            if ui
                                .selectable_value(command, option, media_label(language, option))
                                .changed()
                            {
                                changed = true;
                            }
                        }
                    });
            });
        }
        Action::Launch { path } => {
            ui.horizontal_wrapped(|ui| {
                muted_label(ui, language, TextKey::ActionPath);
                let buffer_id = ui.id().with(("action-path", key));
                let mut text = ui
                    .data_mut(|data| data.get_temp::<String>(buffer_id))
                    .unwrap_or_else(|| path.to_string_lossy().into_owned());
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut text)
                            .desired_width(320.0)
                            .hint_text("…"),
                    )
                    .changed()
                {
                    *path = PathBuf::from(&text);
                    changed = true;
                }
                ui.data_mut(|data| data.insert_temp(buffer_id, text));
            });
        }
    }

    changed
}

fn muted_label(ui: &mut egui::Ui, language: Language, key: TextKey) {
    ui.label(
        egui::RichText::new(tr(language, key))
            .size(12.5)
            .color(theme::muted(ui)),
    );
}

fn control_label(language: Language, control: Control) -> &'static str {
    match control {
        Control::Left => tr(language, TextKey::ControlLeft),
        Control::Right => tr(language, TextKey::ControlRight),
        Control::Press => tr(language, TextKey::ControlPress),
    }
}

fn direction_label(language: Language, direction: VolumeDirection) -> &'static str {
    match direction {
        VolumeDirection::Up => tr(language, TextKey::DirectionUp),
        VolumeDirection::Down => tr(language, TextKey::DirectionDown),
    }
}

fn axis_label(language: Language, axis: ScrollAxis) -> &'static str {
    match axis {
        ScrollAxis::Vertical => tr(language, TextKey::AxisVertical),
        ScrollAxis::Horizontal => tr(language, TextKey::AxisHorizontal),
    }
}

fn media_label(language: Language, command: MediaCommand) -> &'static str {
    match command {
        MediaCommand::PlayPause => tr(language, TextKey::ActionMediaPlayPause),
        MediaCommand::Next => tr(language, TextKey::ActionMediaNext),
        MediaCommand::Previous => tr(language, TextKey::ActionMediaPrevious),
        MediaCommand::Stop => tr(language, TextKey::ActionMediaStop),
    }
}
