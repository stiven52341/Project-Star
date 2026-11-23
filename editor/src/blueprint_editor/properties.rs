//! The inspector of the selected node: what it does, changed with fields.
//! A change that takes pins away also takes their wires.

use eframe::egui;
use star_blueprint::{Call, CallTarget, Graph, Literal, NodeId, NodeKind, Operator};

use super::edit;
use super::literal_field::literal_field;
use super::names::BlueprintNames;
use crate::style::PROPERTY_SPACING;
use crate::texts;

/// The first name `+` gives to an argument is `arg1`, then `arg2`…
const NEW_ARGUMENT_PREFIX: &str = "arg";

/// The fields of the node `id`. Returns whether the graph changed.
pub(super) fn show_node_properties(
    ui: &mut egui::Ui,
    graph: &mut Graph,
    id: NodeId,
    names: &BlueprintNames,
) -> bool {
    let Some(node) = graph.nodes.iter_mut().find(|node| node.id == id) else {
        return false;
    };
    ui.horizontal(|ui| {
        ui.strong(texts::current().node_kind_name(&node.kind));
        ui.weak(texts::current().node_number(id.0));
    });
    let mut renamed_pins = Vec::new();
    let changed = egui::Grid::new("node_properties")
        .num_columns(2)
        .spacing(PROPERTY_SPACING)
        .show(ui, |ui| match &mut node.kind {
            NodeKind::Literal { value } => literal_row(ui, value),
            NodeKind::GetVariable { variable } | NodeKind::SetVariable { variable } => {
                variable_field(ui, variable, names)
            }
            NodeKind::Call(call) => call_fields(ui, call, &mut renamed_pins),
            NodeKind::Operator { operator } => operator_field(ui, operator),
            NodeKind::Entry | NodeKind::Branch | NodeKind::Return => false,
        })
        .inner;
    if let NodeKind::Entry = node.kind {
        ui.weak(texts::current().entry_hint);
    }
    for (old, new) in renamed_pins {
        edit::rename_pin(graph, id, &old, &new);
    }
    if changed {
        edit::remove_broken_links(graph);
    }
    changed
}

fn literal_row(ui: &mut egui::Ui, value: &mut Literal) -> bool {
    ui.weak(texts::current().literal_value);
    let changed = literal_field(ui, "node_literal", value);
    ui.end_row();
    changed
}

/// The variable's name, typed or picked from the blueprint's.
fn variable_field(ui: &mut egui::Ui, variable: &mut String, names: &BlueprintNames) -> bool {
    ui.weak(texts::current().variable);
    let mut changed = false;
    ui.horizontal(|ui| {
        changed |= ui.text_edit_singleline(variable).changed();
        if !names.variables.is_empty() {
            ui.menu_button("▾", |ui| {
                for name in &names.variables {
                    if ui.button(name).clicked() {
                        name.clone_into(variable);
                        changed = true;
                    }
                }
            });
        }
    });
    ui.end_row();
    changed
}

/// What a call calls and what it takes. Arguments renamed are put in
/// `renamed_pins`, so their wires follow them.
fn call_fields(
    ui: &mut egui::Ui,
    call: &mut Call,
    renamed_pins: &mut Vec<(String, String)>,
) -> bool {
    let mut changed = call_target_fields(ui, &mut call.target);
    ui.weak(texts::current().method);
    changed |= ui.text_edit_singleline(&mut call.method).changed();
    ui.end_row();
    ui.weak(texts::current().arguments);
    changed |= arguments_field(ui, &mut call.arguments, renamed_pins);
    ui.end_row();
    ui.weak(texts::current().returns_value);
    changed |= ui.checkbox(&mut call.returns_value, "").changed();
    ui.end_row();
    ui.weak(texts::current().pure_call);
    changed |= ui
        .checkbox(&mut call.pure, "")
        .on_hover_text(texts::current().pure_call_hint)
        .changed();
    ui.end_row();
    changed
}

fn call_target_fields(ui: &mut egui::Ui, target: &mut CallTarget) -> bool {
    ui.weak(texts::current().call_target);
    let mut changed = false;
    egui::ComboBox::from_id_salt("call_target")
        .selected_text(texts::current().call_target_name(target))
        .show_ui(ui, |ui| {
            let options = [
                CallTarget::OwnObject,
                CallTarget::Value,
                CallTarget::Class(String::new()),
            ];
            for option in options {
                let same_kind = std::mem::discriminant(&option) == std::mem::discriminant(target);
                if ui
                    .selectable_label(same_kind, texts::current().call_target_name(&option))
                    .clicked()
                    && !same_kind
                {
                    *target = option;
                    changed = true;
                }
            }
        });
    ui.end_row();
    if let CallTarget::Class(class) = target {
        ui.weak(texts::current().class);
        changed |= ui.text_edit_singleline(class).changed();
        ui.end_row();
    }
    changed
}

/// One field per argument, with `×` to remove it, and `+` to add one.
fn arguments_field(
    ui: &mut egui::Ui,
    arguments: &mut Vec<String>,
    renamed_pins: &mut Vec<(String, String)>,
) -> bool {
    let mut changed = false;
    let mut removed = None;
    ui.vertical(|ui| {
        for (index, argument) in arguments.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                let old = argument.clone();
                if ui.text_edit_singleline(argument).changed() {
                    renamed_pins.push((old, argument.clone()));
                    changed = true;
                }
                if ui
                    .small_button("×")
                    .on_hover_text(texts::current().remove)
                    .clicked()
                {
                    removed = Some(index);
                }
            });
        }
        if ui
            .small_button("+")
            .on_hover_text(texts::current().add_argument)
            .clicked()
        {
            let taken: Vec<&str> = arguments.iter().map(String::as_str).collect();
            let name = edit::unused_name(NEW_ARGUMENT_PREFIX, &taken);
            arguments.push(name);
            changed = true;
        }
    });
    if let Some(index) = removed {
        arguments.remove(index);
        changed = true;
    }
    changed
}

fn operator_field(ui: &mut egui::Ui, operator: &mut Operator) -> bool {
    ui.weak(texts::current().menu_operator);
    let mut changed = false;
    egui::ComboBox::from_id_salt("operator")
        .selected_text(texts::current().operator_name(*operator))
        .show_ui(ui, |ui| {
            for option in Operator::ALL {
                changed |= ui
                    .selectable_value(operator, option, texts::current().operator_name(option))
                    .changed();
            }
        });
    ui.end_row();
    changed
}
