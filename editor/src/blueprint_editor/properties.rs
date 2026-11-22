//! The inspector of the selected node: what it does, changed with fields.
//! A change that takes pins away also takes their wires.

use eframe::egui;
use star_blueprint::{Call, CallTarget, Graph, Literal, NodeId, NodeKind, Operator};

use super::edit;
use super::names::{BlueprintNames, LiteralKind};
use crate::texts;

const PROPERTY_SPACING: [f32; 2] = [12.0, 8.0];
/// How much a decimal changes per point dragged.
const DECIMAL_DRAG_SPEED: f64 = 0.1;
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
        ui.strong(texts::node_kind_name(&node.kind));
        ui.weak(texts::node_number(id.0));
    });
    let mut renamed_pins = Vec::new();
    let changed = egui::Grid::new("node_properties")
        .num_columns(2)
        .spacing(PROPERTY_SPACING)
        .show(ui, |ui| match &mut node.kind {
            NodeKind::Literal { value } => literal_fields(ui, value),
            NodeKind::GetVariable { variable } | NodeKind::SetVariable { variable } => {
                variable_field(ui, variable, names)
            }
            NodeKind::Call(call) => call_fields(ui, call, &mut renamed_pins),
            NodeKind::Operator { operator } => operator_field(ui, operator),
            NodeKind::Entry | NodeKind::Branch | NodeKind::Return => false,
        })
        .inner;
    if let NodeKind::Entry = node.kind {
        ui.weak(texts::ENTRY_HINT);
    }
    for (old, new) in renamed_pins {
        edit::rename_pin(graph, id, &old, &new);
    }
    if changed {
        edit::remove_broken_links(graph);
    }
    changed
}

fn literal_fields(ui: &mut egui::Ui, value: &mut Literal) -> bool {
    ui.weak(texts::LITERAL_KIND);
    let current = LiteralKind::of(value);
    let mut kind = current;
    egui::ComboBox::from_id_salt("literal_kind")
        .selected_text(kind.name())
        .show_ui(ui, |ui| {
            for option in LiteralKind::ALL {
                ui.selectable_value(&mut kind, option, option.name());
            }
        });
    ui.end_row();
    if kind != current {
        *value = kind.default_value();
        return true;
    }
    ui.weak(texts::LITERAL_VALUE);
    let response = match value {
        Literal::Bool(value) => ui.checkbox(value, ""),
        Literal::Int(value) => ui.add(egui::DragValue::new(value)),
        Literal::Float(value) => ui.add(egui::DragValue::new(value).speed(DECIMAL_DRAG_SPEED)),
        Literal::String(text) => ui.text_edit_singleline(text),
    };
    ui.end_row();
    response.changed()
}

/// The variable's name, typed or picked from the blueprint's.
fn variable_field(ui: &mut egui::Ui, variable: &mut String, names: &BlueprintNames) -> bool {
    ui.weak(texts::VARIABLE);
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
    ui.weak(texts::METHOD);
    changed |= ui.text_edit_singleline(&mut call.method).changed();
    ui.end_row();
    ui.weak(texts::ARGUMENTS);
    changed |= arguments_field(ui, &mut call.arguments, renamed_pins);
    ui.end_row();
    ui.weak(texts::RETURNS_VALUE);
    changed |= ui.checkbox(&mut call.returns_value, "").changed();
    ui.end_row();
    ui.weak(texts::PURE_CALL);
    changed |= ui
        .checkbox(&mut call.pure, "")
        .on_hover_text(texts::PURE_CALL_HINT)
        .changed();
    ui.end_row();
    changed
}

fn call_target_fields(ui: &mut egui::Ui, target: &mut CallTarget) -> bool {
    ui.weak(texts::CALL_TARGET);
    let mut changed = false;
    egui::ComboBox::from_id_salt("call_target")
        .selected_text(texts::call_target_name(target))
        .show_ui(ui, |ui| {
            let options = [
                CallTarget::OwnObject,
                CallTarget::Value,
                CallTarget::Class(String::new()),
            ];
            for option in options {
                let same_kind = std::mem::discriminant(&option) == std::mem::discriminant(target);
                if ui
                    .selectable_label(same_kind, texts::call_target_name(&option))
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
        ui.weak(texts::CLASS);
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
                if ui.small_button("×").on_hover_text(texts::REMOVE).clicked() {
                    removed = Some(index);
                }
            });
        }
        if ui
            .small_button("+")
            .on_hover_text(texts::ADD_ARGUMENT)
            .clicked()
        {
            let name = new_argument_name(arguments);
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

/// `arg1`, or the first `argN` no argument has. Among one more number than
/// there are arguments, one is always free.
fn new_argument_name(arguments: &[String]) -> String {
    (1..=arguments.len() + 1)
        .map(|number| format!("{NEW_ARGUMENT_PREFIX}{number}"))
        .find(|name| !arguments.contains(name))
        .unwrap_or_default()
}

fn operator_field(ui: &mut egui::Ui, operator: &mut Operator) -> bool {
    ui.weak(texts::MENU_OPERATOR);
    let mut changed = false;
    egui::ComboBox::from_id_salt("operator")
        .selected_text(texts::operator_name(*operator))
        .show_ui(ui, |ui| {
            for option in Operator::ALL {
                changed |= ui
                    .selectable_value(operator, option, texts::operator_name(option))
                    .changed();
            }
        });
    ui.end_row();
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_arguments_get_a_name_no_other_has() {
        assert_eq!(new_argument_name(&[]), "arg1");
        assert_eq!(
            new_argument_name(&["arg1".to_owned(), "arg3".to_owned()]),
            "arg2"
        );
    }
}
