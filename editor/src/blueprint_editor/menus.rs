//! The menu of the canvas's right button: add a node where it was opened,
//! delete the node under it or remove the wires of a pin.

use eframe::egui::{self, Pos2};
use star_blueprint::{Call, CallTarget, Graph, NodeId, NodeKind, Operator, PinRef};

use super::edit;
use super::names::{BlueprintNames, LiteralKind};
use super::selection::Selection;
use crate::texts;

/// What was under the pointer when the menu was opened.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum MenuTarget {
    Pin(PinRef),
    Node(NodeId),
    /// The empty canvas, at this point of the graph.
    Background(Pos2),
}

/// The menu's entries for `target`. Returns whether the graph changed.
pub(super) fn show_menu(
    ui: &mut egui::Ui,
    graph: &mut Graph,
    target: &MenuTarget,
    names: &BlueprintNames,
    selection: &mut Selection,
) -> bool {
    match target {
        MenuTarget::Pin(pin) => {
            let connected = edit::is_connected(graph, pin);
            if ui
                .add_enabled(
                    connected,
                    egui::Button::new(texts::current().disconnect_pin),
                )
                .clicked()
            {
                edit::disconnect(graph, pin);
                ui.close();
                return true;
            }
            false
        }
        MenuTarget::Node(id) => {
            // The node right-clicked is in the selection: the menu acts on
            // all of it.
            let only_entry = selection.nodes().iter().all(|selected| {
                graph
                    .node(*selected)
                    .is_none_or(|node| matches!(node.kind, NodeKind::Entry))
            });
            let label = if selection.len() > 1 {
                texts::current().delete_nodes
            } else {
                texts::current().delete_node
            };
            let button = ui
                .add_enabled(
                    !only_entry && selection.contains(*id),
                    egui::Button::new(label),
                )
                .on_disabled_hover_text(texts::current().entry_cannot_be_deleted);
            if button.clicked() {
                let removed = edit::remove_nodes(graph, selection.nodes());
                selection.clear();
                ui.close();
                return removed;
            }
            false
        }
        MenuTarget::Background(position) => match new_node_menu(ui, names) {
            Some(kind) => {
                *selection = Selection::only(edit::add_node(graph, kind, *position));
                ui.close();
                true
            }
            None => false,
        },
    }
}

/// The kinds of node that can be added; returns the one chosen.
fn new_node_menu(ui: &mut egui::Ui, names: &BlueprintNames) -> Option<NodeKind> {
    let mut chosen = None;
    ui.weak(texts::current().add_node);
    ui.menu_button(texts::current().menu_value, |ui| {
        for kind in LiteralKind::ALL {
            if ui.button(kind.name()).clicked() {
                chosen = Some(NodeKind::Literal {
                    value: kind.default_value(),
                });
            }
        }
    });
    ui.menu_button(texts::current().menu_get_variable, |ui| {
        if let Some(variable) = variable_menu(ui, names) {
            chosen = Some(NodeKind::GetVariable { variable });
        }
    });
    ui.menu_button(texts::current().menu_set_variable, |ui| {
        if let Some(variable) = variable_menu(ui, names) {
            chosen = Some(NodeKind::SetVariable { variable });
        }
    });
    ui.menu_button(texts::current().menu_call, |ui| {
        if let Some(call) = call_menu(ui, names) {
            chosen = Some(NodeKind::Call(call));
        }
    });
    ui.menu_button(texts::current().menu_operator, |ui| {
        for operator in Operator::ALL {
            if ui
                .button(texts::current().operator_name(operator))
                .clicked()
            {
                chosen = Some(NodeKind::Operator { operator });
            }
        }
    });
    if ui.button(texts::current().node_branch).clicked() {
        chosen = Some(NodeKind::Branch);
    }
    if ui.button(texts::current().node_return).clicked() {
        chosen = Some(NodeKind::Return);
    }
    chosen
}

/// The blueprint's variables, and an entry for any other one (of the base
/// class), whose name is written in the inspector.
fn variable_menu(ui: &mut egui::Ui, names: &BlueprintNames) -> Option<String> {
    let mut chosen = None;
    for variable in &names.variables {
        if ui.button(variable).clicked() {
            chosen = Some(variable.clone());
        }
    }
    if !names.variables.is_empty() {
        ui.separator();
    }
    if ui.button(texts::current().other_name).clicked() {
        chosen = Some(String::new());
    }
    chosen
}

/// The blueprint's own methods, and calls to fill in the inspector.
fn call_menu(ui: &mut egui::Ui, names: &BlueprintNames) -> Option<Call> {
    let mut chosen = None;
    for method in &names.methods {
        if ui.button(format!("{}()", method.method)).clicked() {
            chosen = Some(method.clone());
        }
    }
    if !names.methods.is_empty() {
        ui.separator();
    }
    let targets = [
        (texts::current().call_own_method, CallTarget::OwnObject),
        (texts::current().call_method_of_value, CallTarget::Value),
        (
            texts::current().call_class_function,
            CallTarget::Class(String::new()),
        ),
    ];
    for (label, target) in targets {
        if ui.button(label).clicked() {
            chosen = Some(Call {
                target,
                method: String::new(),
                arguments: Vec::new(),
                returns_value: false,
                pure: false,
            });
        }
    }
    chosen
}
