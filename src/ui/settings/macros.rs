//! Macros page: list and edit reusable input sequences.

use crate::core::action::{MouseButton, ScrollAxis};
use crate::core::{KeyChord, KeyCode, Language, MacroDefinition, MacroStep, TextKey, tr};
use crate::ui::settings::{PageContext, warning};
use crate::ui::{theme, widgets};
use eframe::egui;

pub fn show(ui: &mut egui::Ui, context: &mut PageContext<'_>) {
    let language = context.language;
    widgets::page_header(
        ui,
        tr(language, TextKey::TabMacros),
        tr(language, TextKey::MacrosSubtitle),
    );

    let config = context.runtime.config();
    let mut macros = config.macros.clone();
    let mut changed = false;

    let selection_id = ui.id().with("macro-selection");
    let mut selected = ui
        .data(|data| data.get_temp::<usize>(selection_id))
        .filter(|index| *index < macros.len());
    if selected.is_none() && !macros.is_empty() {
        selected = Some(0);
    }

    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.set_width(220.0);
            if widgets::secondary_button(ui, tr(language, TextKey::NewMacroButton)).clicked() {
                macros.push(MacroDefinition {
                    name: unique_macro_name(&macros, language),
                    steps: vec![MacroStep::Delay { ms: 100 }],
                });
                selected = Some(macros.len() - 1);
                changed = true;
            }

            ui.add_space(10.0);
            if macros.is_empty() {
                ui.label(
                    egui::RichText::new(tr(language, TextKey::EmptyMacrosBody))
                        .size(12.0)
                        .color(theme::muted(ui)),
                );
            } else {
                for (index, definition) in macros.iter().enumerate() {
                    if widgets::nav_item(ui, &definition.name, selected == Some(index)).clicked() {
                        selected = Some(index);
                    }
                }
            }

            if let Some(index) = selected {
                ui.add_space(10.0);
                if widgets::secondary_button(ui, tr(language, TextKey::DeleteMacroButton)).clicked()
                {
                    macros.remove(index);
                    selected = if macros.is_empty() {
                        None
                    } else {
                        Some(index.min(macros.len() - 1))
                    };
                    changed = true;
                }
            }
        });

        ui.add_space(14.0);

        ui.vertical(|ui| {
            if let Some(index) = selected
                && let Some(definition) = macros.get_mut(index)
                && editor(ui, language, index, definition)
            {
                changed = true;
            }
        });
    });

    if let Some(index) = selected {
        ui.data_mut(|data| data.insert_temp(selection_id, index));
    }

    if changed
        && let Err(error) = context
            .runtime
            .update_config(|config| config.macros = macros.clone())
    {
        warning(ui, &error.to_string());
    }
}

fn editor(
    ui: &mut egui::Ui,
    language: Language,
    macro_index: usize,
    definition: &mut MacroDefinition,
) -> bool {
    let mut changed = false;

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(tr(language, TextKey::MacroNameLabel))
                .size(12.5)
                .color(theme::muted(ui)),
        );
        let mut name = definition.name.clone();
        if ui
            .add(egui::TextEdit::singleline(&mut name).desired_width(220.0))
            .changed()
        {
            definition.name = name;
            changed = true;
        }
    });

    ui.add_space(10.0);
    ui.label(
        egui::RichText::new(tr(language, TextKey::MacroStepsTitle))
            .font(theme::heading_font(14.0))
            .color(theme::card_text(ui)),
    );
    ui.add_space(6.0);

    let mut remove = None;
    let mut move_up = None;
    let mut move_down = None;
    let step_count = definition.steps.len();

    for (step_index, step) in definition.steps.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("{:02}", step_index + 1))
                    .size(11.5)
                    .color(theme::muted(ui)),
            );
            changed |= step_editor(ui, language, macro_index, step_index, step);

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("✕").clicked() {
                    remove = Some(step_index);
                }
                if step_index + 1 < step_count && ui.small_button("↓").clicked() {
                    move_down = Some(step_index);
                }
                if step_index > 0 && ui.small_button("↑").clicked() {
                    move_up = Some(step_index);
                }
            });
        });
    }

    if let Some(index) = remove {
        definition.steps.remove(index);
        changed = true;
    }
    if let Some(index) = move_up {
        definition.steps.swap(index, index - 1);
        changed = true;
    }
    if let Some(index) = move_down
        && index + 1 < definition.steps.len()
    {
        definition.steps.swap(index, index + 1);
        changed = true;
    }

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        let kind_id = ui.id().with(("step-kind", macro_index));
        let mut kind = ui
            .data(|data| data.get_temp::<StepKind>(kind_id))
            .unwrap_or(StepKind::Tap);
        egui::ComboBox::from_id_salt(("step-kind-combo", macro_index))
            .selected_text(kind.label(language))
            .width(160.0)
            .show_ui(ui, |ui| {
                for option in StepKind::ALL {
                    ui.selectable_value(&mut kind, option, option.label(language));
                }
            });
        if widgets::secondary_button(ui, tr(language, TextKey::AddStepButton)).clicked() {
            definition.steps.push(kind.default_step());
            changed = true;
        }
        ui.data_mut(|data| data.insert_temp(kind_id, kind));
    });

    changed
}

fn step_editor(
    ui: &mut egui::Ui,
    language: Language,
    macro_index: usize,
    step_index: usize,
    step: &mut MacroStep,
) -> bool {
    let mut changed = false;
    let salt = ("step", macro_index, step_index);

    match step {
        MacroStep::Tap { chord } => {
            ui.label(step_label(language, StepKind::Tap));
            changed |= widgets::chord_picker(ui, salt, language, chord);
        }
        MacroStep::Press { chord } => {
            ui.label(step_label(language, StepKind::Press));
            changed |= widgets::chord_picker(ui, salt, language, chord);
        }
        MacroStep::Release { chord } => {
            ui.label(step_label(language, StepKind::Release));
            changed |= widgets::chord_picker(ui, salt, language, chord);
        }
        MacroStep::Text { text } => {
            ui.label(step_label(language, StepKind::Text));
            let mut buffer = text.clone();
            if ui
                .add(egui::TextEdit::singleline(&mut buffer).desired_width(220.0))
                .changed()
            {
                *text = buffer;
                changed = true;
            }
        }
        MacroStep::Delay { ms } => {
            ui.label(step_label(language, StepKind::Delay));
            ui.label(
                egui::RichText::new(tr(language, TextKey::DelayMsLabel))
                    .size(12.0)
                    .color(theme::muted(ui)),
            );
            changed |= ui.add(egui::DragValue::new(ms).range(0..=60_000)).changed();
        }
        MacroStep::Scroll { axis, amount } => {
            ui.label(step_label(language, StepKind::Scroll));
            egui::ComboBox::from_id_salt(("step-axis", macro_index, step_index))
                .selected_text(axis_label(language, *axis))
                .width(110.0)
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
            changed |= ui
                .add(egui::DragValue::new(amount).range(-100..=100))
                .changed();
        }
        MacroStep::MouseClick { button } => {
            ui.label(step_label(language, StepKind::MouseClick));
            egui::ComboBox::from_id_salt(("step-button", macro_index, step_index))
                .selected_text(button.label(language))
                .width(120.0)
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
        }
    }

    changed
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StepKind {
    Tap,
    Press,
    Release,
    Text,
    Delay,
    Scroll,
    MouseClick,
}

impl StepKind {
    const ALL: [Self; 7] = [
        Self::Tap,
        Self::Press,
        Self::Release,
        Self::Text,
        Self::Delay,
        Self::Scroll,
        Self::MouseClick,
    ];

    fn label(self, language: Language) -> egui::RichText {
        step_label(language, self)
    }

    fn default_step(self) -> MacroStep {
        let chord = KeyChord::new(KeyCode::KeyA);
        match self {
            Self::Tap => MacroStep::Tap { chord },
            Self::Press => MacroStep::Press { chord },
            Self::Release => MacroStep::Release { chord },
            Self::Text => MacroStep::Text {
                text: "…".to_owned(),
            },
            Self::Delay => MacroStep::Delay { ms: 100 },
            Self::Scroll => MacroStep::Scroll {
                axis: ScrollAxis::Vertical,
                amount: -3,
            },
            Self::MouseClick => MacroStep::MouseClick {
                button: MouseButton::Left,
            },
        }
    }
}

fn step_label(language: Language, kind: StepKind) -> egui::RichText {
    let key = match kind {
        StepKind::Tap => TextKey::StepTap,
        StepKind::Press => TextKey::StepPress,
        StepKind::Release => TextKey::StepRelease,
        StepKind::Text => TextKey::StepText,
        StepKind::Delay => TextKey::StepDelay,
        StepKind::Scroll => TextKey::StepScroll,
        StepKind::MouseClick => TextKey::StepMouseClick,
    };
    egui::RichText::new(tr(language, key)).size(12.5)
}

fn axis_label(language: Language, axis: ScrollAxis) -> &'static str {
    match axis {
        ScrollAxis::Vertical => tr(language, TextKey::AxisVertical),
        ScrollAxis::Horizontal => tr(language, TextKey::AxisHorizontal),
    }
}

fn unique_macro_name(macros: &[MacroDefinition], language: Language) -> String {
    let base = tr(language, TextKey::TabMacros);
    let mut number = 1;
    loop {
        let candidate = format!("{base} {number}");
        if !macros.iter().any(|definition| definition.name == candidate) {
            return candidate;
        }
        number += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn macro_names_do_not_collide() {
        let existing = vec![
            MacroDefinition {
                name: "Macros 1".to_owned(),
                steps: vec![MacroStep::Delay { ms: 0 }],
            },
            MacroDefinition {
                name: "Macros 2".to_owned(),
                steps: vec![MacroStep::Delay { ms: 0 }],
            },
        ];

        let name = unique_macro_name(&existing, Language::En);
        assert_eq!(name, "Macros 3");
    }

    #[test]
    fn default_steps_are_valid() {
        for kind in StepKind::ALL {
            let step = kind.default_step();
            let definition = MacroDefinition {
                name: "macro".to_owned(),
                steps: vec![step],
            };
            assert_eq!(
                definition.validate(),
                Ok(()),
                "invalid default for {kind:?}"
            );
        }
    }
}
