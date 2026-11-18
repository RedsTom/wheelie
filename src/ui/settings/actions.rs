//! Actions page: manage the action sets shown in the wheel.

use crate::core::action::{MediaCommand, MouseButton, ScrollAxis, VolumeDirection};
use crate::core::profile::Actions;
use crate::core::{
    Action, ActionSet, Config, Control, KeyChord, KeyCode, Language, SetIcon, TextKey, tr,
};
use crate::ui::settings::{PageContext, warning};
use crate::ui::{icons, theme, widgets};
use eframe::egui;
use egui::{Align2, FontId};
use std::path::PathBuf;

pub fn show(ui: &mut egui::Ui, context: &mut PageContext<'_>) {
    let language = context.language;
    widgets::page_header(
        ui,
        tr(language, TextKey::TabActions),
        tr(language, TextKey::ActionsSubtitle),
    );

    let config = context.runtime.config();
    let mut sets = config.action_sets.clone();
    let mut active = config.active_set.clone();
    let mut changed = false;

    let selection_id = ui.id().with("set-selection");
    let mut selected = ui
        .data(|data| data.get_temp::<usize>(selection_id))
        .filter(|index| *index < sets.len());
    if selected.is_none() && !sets.is_empty() {
        selected = Some(0);
    }

    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.set_width(220.0);
            if widgets::secondary_button(ui, tr(language, TextKey::AddSetButton)).clicked() {
                sets.push(new_set(&sets, language));
                selected = Some(sets.len() - 1);
                changed = true;
            }

            ui.add_space(10.0);
            if sets.is_empty() {
                ui.label(
                    egui::RichText::new(tr(language, TextKey::EmptyActionsBody))
                        .size(12.0)
                        .color(theme::muted(ui)),
                );
            } else {
                for (index, set) in sets.iter().enumerate() {
                    let is_active = active.as_deref() == Some(set.id.as_str());
                    if set_row(ui, set, selected == Some(index), is_active) {
                        selected = Some(index);
                    }
                }
            }

            if let Some(index) = selected {
                ui.add_space(10.0);
                if widgets::secondary_button(ui, tr(language, TextKey::DeleteSetButton)).clicked() {
                    let removed = sets.remove(index);
                    if active.as_deref() == Some(removed.id.as_str()) {
                        active = sets.first().map(|set| set.id.clone());
                    }
                    selected = if sets.is_empty() {
                        None
                    } else {
                        Some(index.min(sets.len() - 1))
                    };
                    changed = true;
                }
            }
        });

        ui.add_space(16.0);

        ui.vertical(|ui| {
            if let Some(index) = selected
                && let Some(set) = sets.get_mut(index)
            {
                let is_active = active.as_deref() == Some(set.id.as_str());
                if editor(
                    ui,
                    language,
                    &config,
                    set,
                    is_active,
                    context.platform.brightness_supported,
                ) {
                    changed = true;
                }
            }
        });
    });

    if let Some(index) = selected {
        ui.data_mut(|data| data.insert_temp(selection_id, index));
    }

    if changed {
        let result = context.runtime.update_config(|config| {
            config.action_sets = sets;
            let keep = active
                .filter(|id| config.action_sets.iter().any(|set| &set.id == id))
                .or_else(|| config.action_sets.first().map(|set| set.id.clone()));
            config.active_set = keep;
        });
        if let Err(error) = result {
            warning(ui, &error.to_string());
        }
    }
}

fn editor(
    ui: &mut egui::Ui,
    language: Language,
    config: &Config,
    set: &mut ActionSet,
    is_active: bool,
    brightness_supported: bool,
) -> bool {
    let mut changed = false;

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(tr(language, TextKey::SetNameLabel))
                .size(12.5)
                .color(theme::muted(ui)),
        );
        let mut name = set.name.clone();
        if ui
            .add(egui::TextEdit::singleline(&mut name).desired_width(220.0))
            .changed()
        {
            set.name = name;
            changed = true;
        }
        if is_active {
            ui.add_space(6.0);
            widgets::status_pill(ui, tr(language, TextKey::ActiveSetLabel), theme::SUCCESS);
        }
    });

    ui.add_space(10.0);
    ui.label(
        egui::RichText::new(tr(language, TextKey::SetIconLabel))
            .size(12.5)
            .color(theme::muted(ui)),
    );
    ui.add_space(4.0);
    changed |= icons::picker(ui, &mut set.icon);

    for control in [Control::Left, Control::Press, Control::Right] {
        ui.add_space(10.0);
        ui.separator();
        ui.add_space(8.0);
        changed |= show_control(
            ui,
            language,
            config,
            control,
            &mut set.actions,
            brightness_supported,
        );
    }

    changed
}

fn set_row(ui: &mut egui::Ui, set: &ActionSet, selected: bool, active: bool) -> bool {
    let width = ui.available_width();
    let height = theme::CONTROL_HEIGHT + 6.0;
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click());
    let painter = ui.painter().clone();

    if selected {
        painter.rect_filled(rect, 4, theme::accent_soft());
    } else if response.hovered() {
        painter.rect_filled(rect, 4, ui.visuals().widgets.hovered.weak_bg_fill);
    }

    let color = if selected {
        theme::accent(ui)
    } else {
        ui.visuals().widgets.inactive.fg_stroke.color
    };
    let icon_rect = egui::Rect::from_center_size(
        rect.left_center() + egui::vec2(18.0, 0.0),
        egui::Vec2::splat(18.0),
    );
    icons::paint(&painter, icon_rect, set.icon, color);
    painter.text(
        rect.left_center() + egui::vec2(36.0, 0.0),
        Align2::LEFT_CENTER,
        &set.name,
        FontId::proportional(13.5),
        color,
    );
    if active {
        painter.circle_filled(
            rect.right_center() - egui::vec2(14.0, 0.0),
            4.0,
            theme::SUCCESS,
        );
    }

    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
}

fn new_set(sets: &[ActionSet], language: Language) -> ActionSet {
    let base = tr(language, TextKey::SetDefaultName);
    let mut number = 1;
    loop {
        let name = format!("{base} {number}");
        let id = format!("set-{number}");
        if !sets.iter().any(|set| set.name == name || set.id == id) {
            return ActionSet::new(id, name, SetIcon::Star);
        }
        number += 1;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    None,
    Volume,
    Brightness,
    Scroll,
    Macro,
    Shortcut,
    Mouse,
    Media,
    Launch,
}

impl Kind {
    const ALL: [Self; 9] = [
        Self::None,
        Self::Volume,
        Self::Brightness,
        Self::Scroll,
        Self::Macro,
        Self::Shortcut,
        Self::Mouse,
        Self::Media,
        Self::Launch,
    ];

    /// Kinds offered in the picker. "No action" is always available, and
    /// brightness only when the input backend can change it.
    fn choices(brightness_supported: bool) -> Vec<Self> {
        Self::ALL
            .iter()
            .copied()
            .filter(|kind| brightness_supported || *kind != Self::Brightness)
            .collect()
    }

    fn of(action: &Action) -> Self {
        match action {
            Action::None => Self::None,
            Action::Volume { .. } => Self::Volume,
            Action::Brightness { .. } => Self::Brightness,
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
            Self::Brightness => tr(language, TextKey::ActionKindBrightness),
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
            Self::Brightness => Action::Brightness {
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
    brightness_supported: bool,
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
                .font(theme::heading_font(13.5))
                .color(theme::card_text(ui)),
        );

        let mut kind = Kind::of(actions.for_control(control));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            egui::ComboBox::from_id_salt(("action-kind", control_key))
                .selected_text(kind.label(language))
                .width(170.0)
                .show_ui(ui, |ui| {
                    for option in Kind::choices(brightness_supported) {
                        ui.selectable_value(&mut kind, option, option.label(language));
                    }
                });
        });
        if kind != Kind::of(actions.for_control(control)) {
            *actions.for_control_mut(control) = kind.default_action(config);
            changed = true;
        }
    });

    ui.add_space(6.0);

    let action = actions.for_control_mut(control);
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
        Action::Volume { direction, steps } | Action::Brightness { direction, steps } => {
            ui.horizontal_wrapped(|ui| {
                muted_label(ui, language, TextKey::ActionDirection);
                egui::ComboBox::from_id_salt(("amount-direction", key))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_sets_have_unique_names_and_ids() {
        let mut sets = Vec::new();
        for _ in 0..3 {
            let set = new_set(&sets, Language::En);
            sets.push(set);
        }
        assert_eq!(sets[0].id, "set-1");
        assert_eq!(sets[1].id, "set-2");
        assert_eq!(sets[2].name, "Set 3");
    }

    #[test]
    fn brightness_is_only_offered_when_supported() {
        let without = Kind::choices(false);
        assert!(without.contains(&Kind::None));
        assert!(!without.contains(&Kind::Brightness));

        let with = Kind::choices(true);
        assert!(with.contains(&Kind::Brightness));
        assert_eq!(with.first(), Some(&Kind::None));
    }

    #[test]
    fn kind_round_trips_every_action() {
        let mut config = Config::default();
        config.macros.push(crate::core::MacroDefinition {
            name: "macro".to_owned(),
            steps: vec![crate::core::MacroStep::Delay { ms: 0 }],
        });

        for kind in Kind::ALL {
            let action = kind.default_action(&config);
            assert_eq!(Kind::of(&action), kind, "kind mismatch for {kind:?}");
        }
    }
}
