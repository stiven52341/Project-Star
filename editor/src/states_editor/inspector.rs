//! The inspector of a state machine: what is selected on the graph (a
//! state's name and what it plays, or a transition's ends, fade and
//! conditions) and the values scripts set.

use eframe::egui;
use star_engine::star_scene::{
    AnimationState, AnimationStates, BlendPoint, Condition, Motion, StateTransition, Test,
    ValueKind,
};

use super::edits;
use super::graph::Selected;
use crate::texts;

/// The longest a fade the inspector drags to is, in seconds.
const MAX_FADE: f32 = 10.0;
/// How much dragging a fade changes it.
const FADE_STEP: f64 = 0.01;
/// How much dragging a number of a condition or a blend changes it.
const NUMBER_STEP: f64 = 0.05;
/// Every kind of value, in the order the inspector lists them.
const KINDS: [ValueKind; 3] = [ValueKind::Number, ValueKind::Flag, ValueKind::Trigger];

/// Shows the selection and the values of `machine`; `animations` are the
/// names to choose from. Returns whether the machine changed.
pub(super) fn show_inspector(
    ui: &mut egui::Ui,
    machine: &mut AnimationStates,
    selected: &mut Selected,
    animations: &[String],
) -> bool {
    let mut changed = match *selected {
        Selected::Nothing => {
            ui.weak(texts::current().states_graph_hint());
            false
        }
        Selected::State(index) => state_section(ui, machine, index, animations),
        Selected::Transition(index) => transition_section(ui, machine, index),
    };
    ui.separator();
    changed |= values_section(ui, machine);
    changed
}

fn state_section(
    ui: &mut egui::Ui,
    machine: &mut AnimationStates,
    index: usize,
    animations: &[String],
) -> bool {
    let texts = texts::current();
    let Some(state) = machine.states.get(index) else {
        return false;
    };
    let mut name = state.name.clone();
    let is_start = machine.start == state.name;
    let mut changed = false;
    egui::Grid::new("state_rows").num_columns(2).show(ui, |ui| {
        ui.weak(texts.state_name());
        if ui.text_edit_singleline(&mut name).changed() {
            changed |= edits::rename_state(machine, index, &name);
        }
        ui.end_row();
        ui.weak(texts.start_state());
        let mut start = is_start;
        if ui.checkbox(&mut start, "").changed() && start {
            machine.start.clone_from(&name);
            changed = true;
        }
        ui.end_row();
    });
    let numbers = number_values(machine);
    if let Some(state) = machine.states.get_mut(index) {
        changed |= motion_rows(ui, state, animations, &numbers);
    }
    changed
}

/// What the state plays: one animation, or a blend by a number value.
fn motion_rows(
    ui: &mut egui::Ui,
    state: &mut AnimationState,
    animations: &[String],
    numbers: &[String],
) -> bool {
    let texts = texts::current();
    let mut changed = false;
    let blends = matches!(state.motion, Motion::Blend { .. });
    ui.horizontal(|ui| {
        ui.weak(texts.state_plays());
        if ui
            .selectable_label(!blends, texts.plays_animation())
            .clicked()
            && blends
        {
            let first = first_animation(&state.motion);
            state.motion = Motion::Animation(first);
            changed = true;
        }
        let can_blend = !numbers.is_empty();
        let blend = ui.add_enabled(
            can_blend,
            egui::Button::selectable(blends, texts.plays_blend()),
        );
        let blend = if can_blend {
            blend
        } else {
            blend.on_disabled_hover_text(texts.blend_needs_number())
        };
        if blend.clicked()
            && !blends
            && let Some(first) = numbers.first()
        {
            let animation = first_animation(&state.motion);
            state.motion = Motion::Blend {
                value: first.clone(),
                points: vec![BlendPoint { animation, at: 0.0 }],
            };
            changed = true;
        }
    });
    match &mut state.motion {
        Motion::Animation(animation) => {
            changed |= animation_choice(ui, "state_animation", animation, animations);
        }
        Motion::Blend { value, points } => {
            changed |= blend_rows(ui, value, points, animations, numbers);
        }
    }
    changed
}

/// The animation a state keeps when it changes from one motion to the
/// other.
fn first_animation(motion: &Motion) -> String {
    match motion {
        Motion::Animation(animation) => animation.clone(),
        Motion::Blend { points, .. } => points
            .first()
            .map(|point| point.animation.clone())
            .unwrap_or_default(),
    }
}

fn blend_rows(
    ui: &mut egui::Ui,
    value: &mut String,
    points: &mut Vec<BlendPoint>,
    animations: &[String],
    numbers: &[String],
) -> bool {
    let texts = texts::current();
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.weak(texts.blend_value());
        changed |= name_choice(ui, "blend_value", value, numbers);
    });
    ui.weak(texts.blend_points());
    let mut removed = None;
    for (index, point) in points.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            changed |=
                animation_choice(ui, ("blend_point", index), &mut point.animation, animations);
            ui.weak(texts.blend_at());
            changed |= ui
                .add(egui::DragValue::new(&mut point.at).speed(NUMBER_STEP))
                .changed();
            if ui.small_button("×").on_hover_text(texts.remove()).clicked() {
                removed = Some(index);
            }
        });
    }
    if let Some(index) = removed
        && points.len() > 1
    {
        points.remove(index);
        changed = true;
    }
    if ui.button(texts.add_blend_point()).clicked() {
        let next = edits::next_point(points);
        points.push(next);
        changed = true;
    }
    changed
}

fn transition_section(ui: &mut egui::Ui, machine: &mut AnimationStates, index: usize) -> bool {
    let texts = texts::current();
    let states: Vec<String> = machine
        .states
        .iter()
        .map(|state| state.name.clone())
        .collect();
    let values = machine.values.clone();
    let Some(transition) = machine.transitions.get_mut(index) else {
        return false;
    };
    let mut changed = false;
    egui::Grid::new("transition_rows")
        .num_columns(2)
        .show(ui, |ui| {
            ui.weak(texts.transition_from());
            changed |= from_choice(ui, transition, &states);
            ui.end_row();
            ui.weak(texts.transition_to());
            changed |= name_choice(ui, "transition_to", &mut transition.to, &states);
            ui.end_row();
            ui.weak(texts.transition_fade())
                .on_hover_text(texts.transition_fade_hint());
            changed |= ui
                .add(
                    egui::DragValue::new(&mut transition.fade)
                        .range(0.0..=MAX_FADE)
                        .speed(FADE_STEP)
                        .suffix(" s"),
                )
                .changed();
            ui.end_row();
            ui.weak(texts.transition_at_end())
                .on_hover_text(texts.transition_at_end_hint());
            changed |= ui.checkbox(&mut transition.at_end, "").changed();
            ui.end_row();
        });
    ui.weak(texts.conditions())
        .on_hover_text(texts.conditions_hint());
    changed |= condition_rows(ui, &mut transition.conditions, &values);
    if ui
        .add_enabled(!values.is_empty(), egui::Button::new(texts.add_condition()))
        .on_disabled_hover_text(texts.condition_needs_value())
        .clicked()
        && let Some(condition) = edits::new_condition(machine)
        && let Some(transition) = machine.transitions.get_mut(index)
    {
        transition.conditions.push(condition);
        changed = true;
    }
    changed
}

/// The state a transition leaves, or “Any state”.
fn from_choice(ui: &mut egui::Ui, transition: &mut StateTransition, states: &[String]) -> bool {
    let any = texts::current().any_state();
    let mut changed = false;
    egui::ComboBox::from_id_salt("transition_from")
        .selected_text(transition.from.as_deref().unwrap_or(any))
        .show_ui(ui, |ui| {
            if ui
                .selectable_label(transition.from.is_none(), any)
                .clicked()
                && transition.from.is_some()
            {
                transition.from = None;
                changed = true;
            }
            for state in states {
                let current = transition.from.as_deref() == Some(state.as_str());
                if ui.selectable_label(current, state).clicked() && !current {
                    transition.from = Some(state.clone());
                    changed = true;
                }
            }
        });
    changed
}

fn condition_rows(
    ui: &mut egui::Ui,
    conditions: &mut Vec<Condition>,
    values: &[star_engine::star_scene::StateValue],
) -> bool {
    let names: Vec<String> = values.iter().map(|value| value.name.clone()).collect();
    let mut changed = false;
    let mut removed = None;
    for (index, condition) in conditions.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            if name_choice(ui, ("condition", index), &mut condition.value, &names) {
                if let Some(value) = values.iter().find(|value| value.name == condition.value)
                    && edits::kind_tested(condition.test) != value.kind
                {
                    condition.test = edits::default_test(value.kind);
                }
                changed = true;
            }
            changed |= test_choice(ui, index, &mut condition.test);
            if ui
                .small_button("×")
                .on_hover_text(texts::current().remove())
                .clicked()
            {
                removed = Some(index);
            }
        });
    }
    if let Some(index) = removed {
        conditions.remove(index);
        changed = true;
    }
    changed
}

/// What a condition tests, as its value's kind allows.
fn test_choice(ui: &mut egui::Ui, index: usize, test: &mut Test) -> bool {
    let texts = texts::current();
    match *test {
        Test::Above(limit) | Test::Below(limit) => {
            let above = matches!(test, Test::Above(_));
            let mut limit = limit;
            let mut chosen_above = above;
            egui::ComboBox::from_id_salt(("condition_test", index))
                .selected_text(if above { texts.above() } else { texts.below() })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut chosen_above, true, texts.above());
                    ui.selectable_value(&mut chosen_above, false, texts.below());
                });
            let dragged = ui
                .add(egui::DragValue::new(&mut limit).speed(NUMBER_STEP))
                .changed();
            *test = if chosen_above {
                Test::Above(limit)
            } else {
                Test::Below(limit)
            };
            dragged || chosen_above != above
        }
        Test::Is(mut flag) => {
            ui.weak(texts.is());
            let changed = ui.checkbox(&mut flag, "").changed();
            *test = Test::Is(flag);
            changed
        }
        Test::Triggered => {
            ui.weak(texts.triggered());
            false
        }
    }
}

fn values_section(ui: &mut egui::Ui, machine: &mut AnimationStates) -> bool {
    let texts = texts::current();
    ui.strong(texts.state_values())
        .on_hover_text(texts.state_values_hint());
    let mut changed = false;
    let mut removed = None;
    for index in 0..machine.values.len() {
        let Some(value) = machine.values.get(index) else {
            break;
        };
        let mut name = value.name.clone();
        let mut kind = value.kind;
        ui.horizontal(|ui| {
            if ui
                .add(
                    egui::TextEdit::singleline(&mut name).desired_width(ui.available_width() / 2.0),
                )
                .changed()
            {
                changed |= edits::rename_value(machine, index, &name);
            }
            egui::ComboBox::from_id_salt(("value_kind", index))
                .selected_text(texts.value_kind(kind))
                .show_ui(ui, |ui| {
                    for choice in KINDS {
                        ui.selectable_value(&mut kind, choice, texts.value_kind(choice));
                    }
                });
            if ui.small_button("×").on_hover_text(texts.remove()).clicked() {
                removed = Some(index);
            }
        });
        if machine
            .values
            .get(index)
            .is_some_and(|value| value.kind != kind)
        {
            edits::set_value_kind(machine, index, kind);
            changed = true;
        }
    }
    if let Some(index) = removed {
        edits::delete_value(machine, index);
        changed = true;
    }
    ui.menu_button(texts.add_state_value(), |ui| {
        for kind in KINDS {
            if ui.button(texts.value_kind(kind)).clicked() {
                edits::add_value(machine, kind);
                changed = true;
                ui.close();
            }
        }
    });
    changed
}

fn number_values(machine: &AnimationStates) -> Vec<String> {
    machine
        .values
        .iter()
        .filter(|value| value.kind == ValueKind::Number)
        .map(|value| value.name.clone())
        .collect()
}

/// A combo of `animations`, or a text field if there are none to choose
/// from (the `.anim` file could not be read, or names none yet).
fn animation_choice(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash + std::fmt::Debug,
    animation: &mut String,
    animations: &[String],
) -> bool {
    if animations.is_empty() {
        return ui.text_edit_singleline(animation).changed();
    }
    name_choice(ui, id, animation, animations)
}

/// A combo that chooses `chosen` among `names`.
fn name_choice(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash + std::fmt::Debug,
    chosen: &mut String,
    names: &[String],
) -> bool {
    let mut changed = false;
    egui::ComboBox::from_id_salt(id)
        .selected_text(chosen.as_str())
        .show_ui(ui, |ui| {
            for name in names {
                let current = chosen == name;
                if ui.selectable_label(current, name).clicked() && !current {
                    chosen.clone_from(name);
                    changed = true;
                }
            }
        });
    changed
}
