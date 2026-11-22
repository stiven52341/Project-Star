//! The menu of the canvas's right button: add a node where it was opened,
//! delete the node under it or remove the wires of a pin.

use eframe::egui::{self, Pos2};
use star_blueprint::{Call, CallTarget, Graph, NodeId, NodeKind, Operator, PinRef};

use super::edit;
use super::names::{BlueprintNames, LiteralKind};
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
    selected: &mut Option<NodeId>,
) -> bool {
    match target {
        MenuTarget::Pin(pin) => {
            let connected = edit::is_connected(graph, pin);
            if ui
                .add_enabled(connected, egui::Button::new(texts::DISCONNECT_PIN))
                .clicked()
            {
                edit::disconnect(graph, pin);
                ui.close();
                return true;
            }
            false
        }
        MenuTarget::Node(id) => {
            let is_entry = graph
                .node(*id)
                .is_some_and(|node| matches!(node.kind, NodeKind::Entry));
            let button = ui
                .add_enabled(!is_entry, egui::Button::new(texts::DELETE_NODE))
                .on_disabled_hover_text(texts::ENTRY_CANNOT_BE_DELETED);
            if button.clicked() {
                edit::remove_node(graph, *id);
                *selected = None;
                ui.close();
                return true;
            }
            false
        }
        MenuTarget::Background(position) => match new_node_menu(ui, names) {
            Some(kind) => {
                *selected = Some(edit::add_node(graph, kind, *position));
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
    ui.weak(texts::ADD_NODE);
    ui.menu_button(texts::MENU_VALUE, |ui| {
        for kind in LiteralKind::ALL {
            if ui.button(kind.name()).clicked() {
                chosen = Some(NodeKind::Literal {
                    value: kind.default_value(),
                });
            }
        }
    });
    ui.menu_button(texts::MENU_GET_VARIABLE, |ui| {
        if let Some(variable) = variable_menu(ui, names) {
            chosen = Some(NodeKind::GetVariable { variable });
        }
    });
    ui.menu_button(texts::MENU_SET_VARIABLE, |ui| {
        if let Some(variable) = variable_menu(ui, names) {
            chosen = Some(NodeKind::SetVariable { variable });
        }
    });
    ui.menu_button(texts::MENU_CALL, |ui| {
        if let Some(call) = call_menu(ui, names) {
            chosen = Some(NodeKind::Call(call));
        }
    });
    ui.menu_button(texts::MENU_OPERATOR, |ui| {
        for operator in Operator::ALL {
            if ui.button(texts::operator_name(operator)).clicked() {
                chosen = Some(NodeKind::Operator { operator });
            }
        }
    });
    if ui.button(texts::NODE_BRANCH).clicked() {
        chosen = Some(NodeKind::Branch);
    }
    if ui.button(texts::NODE_RETURN).clicked() {
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
    if ui.button(texts::OTHER_NAME).clicked() {
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
        (texts::CALL_OWN_METHOD, CallTarget::OwnObject),
        (texts::CALL_METHOD_OF_VALUE, CallTarget::Value),
        (texts::CALL_CLASS_FUNCTION, CallTarget::Class(String::new())),
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
