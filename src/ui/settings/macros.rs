//! Macros page: list and edit reusable input sequences.

use crate::core::action::{MouseButton, ScrollAxis};
use crate::core::{KeyChord, KeyCode, Language, MacroDefinition, MacroStep, TextKey, tr};
use crate::ui::settings::{PageContext, warning};
use crate::ui::{theme, widgets};
use eframe::egui;
use egui_phosphor::regular as ph;

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
            if widgets::secondary_button(
                ui,
                &format!("{}  {}", ph::PLUS, tr(language, TextKey::NewMacroButton)),
            )
            .clicked()
            {
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
                if widgets::secondary_button(
                    ui,
                    &format!(
                        "{}  {}",
                        ph::TRASH,
                        tr(language, TextKey::DeleteMacroButton)
                    ),
                )
                .clicked()
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

        ui.add_space(16.0);

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
            .add(egui::TextEdit::singleline(&mut name).desired_width(240.0))
            .changed()
        {
            definition.name = name;
            changed = true;
        }
    });

    ui.add_space(12.0);
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(tr(language, TextKey::MacroStepsTitle))
                .font(theme::heading_font(14.0))
                .color(theme::card_text(ui)),
        );
        ui.label(
            egui::RichText::new(definition.steps.len().to_string())
                .size(11.5)
                .color(theme::muted(ui)),
        );
    });
    ui.add_space(6.0);

    if definition.steps.is_empty() {
        ui.label(
            egui::RichText::new(tr(language, TextKey::EmptyStepsLabel))
                .size(12.5)
                .color(theme::muted(ui)),
        );
        ui.add_space(6.0);
    }

    let mut remove = None;
    let mut move_up = None;
    let mut move_down = None;
    let step_count = definition.steps.len();

    for (step_index, step) in definition.steps.iter_mut().enumerate() {
        egui::Frame::NONE
            .fill(ui.visuals().faint_bg_color)
            .stroke(egui::Stroke::new(1.0_f32, theme::border(ui)))
            .corner_radius(6)
            .inner_margin(egui::Margin::symmetric(10, 7))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("{:02}", step_index + 1))
                            .size(11.0)
                            .color(theme::muted(ui)),
                    );
                    changed |= step_editor(ui, language, macro_index, step_index, step);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if icon_button(ui, ph::TRASH, tr(language, TextKey::RemoveStepButton)) {
                            remove = Some(step_index);
                        }
                        if step_index + 1 < step_count
                            && icon_button(
                                ui,
                                ph::ARROW_DOWN,
                                tr(language, TextKey::MoveStepDownButton),
                            )
                        {
                            move_down = Some(step_index);
                        }
                        if step_index > 0
                            && icon_button(
                                ui,
                                ph::ARROW_UP,
                                tr(language, TextKey::MoveStepUpButton),
                            )
                        {
                            move_up = Some(step_index);
                        }
                    });
                });
            });
        ui.add_space(6.0);
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

    ui.add_space(10.0);
    ui.label(
        egui::RichText::new(tr(language, TextKey::AddStepButton))
            .font(theme::heading_font(13.0))
            .color(theme::card_text(ui)),
    );
    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        for kind in StepKind::ALL {
            let label = format!("{}  {}", kind.icon(), kind.text(language));
            if widgets::secondary_button(ui, &label).clicked() {
                definition.steps.push(kind.default_step());
                changed = true;
            }
        }
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
    let kind = StepKind::of(step);

    ui.label(
        egui::RichText::new(format!("{}  {}", kind.icon(), kind.text(language)))
            .size(12.0)
            .color(theme::card_text(ui)),
    );

    match step {
        MacroStep::Tap { chord } => {
            changed |= widgets::chord_picker(ui, salt, language, chord);
        }
        MacroStep::Press { chord } => {
            changed |= widgets::chord_picker(ui, salt, language, chord);
        }
        MacroStep::Release { chord } => {
            changed |= widgets::chord_picker(ui, salt, language, chord);
        }
        MacroStep::Text { text } => {
            let mut buffer = text.clone();
            if ui
                .add(egui::TextEdit::singleline(&mut buffer).desired_width(240.0))
                .changed()
            {
                *text = buffer;
                changed = true;
            }
        }
        MacroStep::Delay { ms } => {
            ui.label(
                egui::RichText::new(tr(language, TextKey::DelayMsLabel))
                    .size(12.0)
                    .color(theme::muted(ui)),
            );
            changed |= ui.add(egui::DragValue::new(ms).range(0..=60_000)).changed();
        }
        MacroStep::Scroll { axis, amount } => {
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

fn icon_button(ui: &mut egui::Ui, glyph: &str, tooltip: &str) -> bool {
    ui.add(egui::Button::new(egui::RichText::new(glyph).size(13.0)).frame(false))
        .on_hover_text(tooltip.to_owned())
        .clicked()
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

    fn of(step: &MacroStep) -> Self {
        match step {
            MacroStep::Tap { .. } => Self::Tap,
            MacroStep::Press { .. } => Self::Press,
            MacroStep::Release { .. } => Self::Release,
            MacroStep::Text { .. } => Self::Text,
            MacroStep::Delay { .. } => Self::Delay,
            MacroStep::Scroll { .. } => Self::Scroll,
            MacroStep::MouseClick { .. } => Self::MouseClick,
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Self::Tap => ph::HAND_TAP,
            Self::Press => ph::ARROW_DOWN,
            Self::Release => ph::ARROW_UP,
            Self::Text => ph::TEXT_T,
            Self::Delay => ph::CLOCK,
            Self::Scroll => ph::ARROWS_VERTICAL,
            Self::MouseClick => ph::MOUSE,
        }
    }

    fn text(self, language: Language) -> &'static str {
        let key = match self {
            Self::Tap => TextKey::StepTap,
            Self::Press => TextKey::StepPress,
            Self::Release => TextKey::StepRelease,
            Self::Text => TextKey::StepText,
            Self::Delay => TextKey::StepDelay,
            Self::Scroll => TextKey::StepScroll,
            Self::MouseClick => TextKey::StepMouseClick,
        };
        tr(language, key)
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
    fn default_steps_are_valid_and_map_back() {
        for kind in StepKind::ALL {
            let step = kind.default_step();
            assert_eq!(StepKind::of(&step), kind);
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
