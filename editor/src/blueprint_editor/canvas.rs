//! The canvas of a graph and what the mouse does on it:
//!
//! - dragging the background draws a box that selects the nodes it
//!   touches; clicking a node selects it, and `Shift` or `Ctrl` add to the
//!   selection (a click on a selected node then leaves it out);
//! - dragging a selected node moves every selected node;
//! - dragging from a pin to another one joins them with a wire; clicking a
//!   wire selects it;
//! - dragging with the middle button, or scrolling with two fingers on a
//!   touchpad, moves the view; the mouse wheel, `Ctrl` + wheel and pinching
//!   zoom;
//! - the right button opens a menu to add nodes, delete them or remove
//!   wires; `Delete` deletes the selected nodes or the selected wire.
//!
//! Nodes with mistakes found by “Compile” have a red border, and the
//! pointer over them shows the mistakes.

use std::collections::HashMap;

use eframe::egui::{self, Key, Modifiers, PointerButton, Pos2, Rect, Response, Sense};
use star_blueprint::{Graph, NodeId, PinRef};

use super::edit;
use super::layout::{graph_layout, link_at, node_at, nodes_in, pin_at};
use super::menus::{MenuTarget, show_menu};
use super::names::BlueprintNames;
use super::painting::Painting;
use super::selection::Selection;
use crate::canvas_view::{CanvasView, scroll_view};

/// What the canvas of one blueprint remembers between frames.
#[derive(Default)]
pub(super) struct CanvasState {
    pub(super) view: CanvasView,
    /// The selected nodes; the inspector shows the node if it is only one.
    pub(super) selection: Selection,
    drag: Option<Drag>,
    /// What the open menu of the right button is about.
    menu: Option<MenuTarget>,
    /// A node to bring to the middle of the canvas, once its size is known.
    focus: Option<NodeId>,
}

/// The mistakes of each node of the graph shown.
pub(super) type NodeMistakes<'problems> = HashMap<NodeId, Vec<&'problems str>>;

/// What a drag with the main button that is going on does.
enum Drag {
    /// Moves the selected nodes.
    Nodes,
    Wire(PinRef),
    /// Selects the nodes in the box from `start` (in the graph) to the
    /// pointer, besides those `kept`.
    Box {
        start: Pos2,
        kept: Selection,
    },
}

impl CanvasState {
    /// Forgets what was about the graph shown, to show another one.
    pub(super) fn show_other_graph(&mut self) {
        *self = Self::default();
    }

    /// Selects the node `id` and brings it to the middle of the canvas.
    pub(super) fn focus_on(&mut self, id: NodeId) {
        self.selection = Selection::only(id);
        self.focus = Some(id);
    }
}

/// Draws `graph` filling the space left in `ui` and applies what the mouse
/// and keyboard do to it. Returns whether the graph changed.
pub(super) fn show_graph(
    ui: &mut egui::Ui,
    graph: &mut Graph,
    state: &mut CanvasState,
    names: &BlueprintNames,
    mistakes: &NodeMistakes<'_>,
) -> bool {
    let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
    if let Some(id) = state.focus.take()
        && let Some(node) = graph.node(id)
    {
        let center = super::layout::NodeLayout::new(node, graph).rect.center();
        state.view.look_at(center, rect.size());
    }
    state.selection.keep_existing(graph);
    let selection_before = state.selection.clone();
    let mut changed = handle_pointer(ui, graph, state, &response, rect.min);
    scroll_view(ui, &mut state.view, &response, rect.min);
    response.context_menu(|ui| {
        if let Some(target) = &state.menu {
            changed |= show_menu(ui, graph, target, names, &mut state.selection);
        }
    });
    changed |= delete_selected_on_key(ui, graph, state);
    if state.selection != selection_before {
        // The inspector, drawn before the canvas, shows the new selection.
        ui.ctx().request_repaint();
    }
    paint(ui, graph, state, &response, rect, mistakes);
    show_mistakes_under_pointer(graph, state, &response, rect.min, mistakes);
    changed
}

/// The mistakes of the node under the pointer, next to it.
fn show_mistakes_under_pointer(
    graph: &Graph,
    state: &CanvasState,
    response: &Response,
    origin: Pos2,
    mistakes: &NodeMistakes<'_>,
) {
    let Some(pointer) = response.hover_pos() else {
        return;
    };
    let layouts = graph_layout(graph);
    let point = state.view.to_graph(origin, pointer);
    if let Some(messages) = node_at(graph, &layouts, point).and_then(|id| mistakes.get(&id)) {
        response
            .clone()
            .on_hover_text_at_pointer(messages.join("\n"));
    }
}

/// Selects, moves nodes, joins wires and moves the view with the mouse's
/// buttons. Returns whether the graph changed.
fn handle_pointer(
    ui: &egui::Ui,
    graph: &mut Graph,
    state: &mut CanvasState,
    response: &Response,
    origin: Pos2,
) -> bool {
    let layouts = graph_layout(graph);
    let view = state.view;
    let at = |screen: Pos2| view.to_graph(origin, screen);
    let adding = ui.input(|input| adds_to_selection(input.modifiers));
    let mut changed = false;
    if response.dragged_by(PointerButton::Middle) {
        state.view.pan_by(response.drag_delta());
    }
    if response.drag_started_by(PointerButton::Primary)
        && let Some(pressed) = ui.input(|input| input.pointer.press_origin())
    {
        let point = at(pressed);
        state.drag = Some(if let Some(pin) = pin_at(graph, &layouts, point) {
            Drag::Wire(pin)
        } else if let Some(id) = node_at(graph, &layouts, point) {
            if !state.selection.contains(id) {
                if adding {
                    state.selection.add(id);
                } else {
                    state.selection = Selection::only(id);
                }
            }
            Drag::Nodes
        } else {
            let kept = if adding {
                state.selection.clone()
            } else {
                Selection::default()
            };
            Drag::Box { start: point, kept }
        });
    }
    if response.dragged_by(PointerButton::Primary) {
        match &state.drag {
            Some(Drag::Nodes) => {
                let delta = response.drag_delta() / state.view.zoom();
                for id in state.selection.nodes() {
                    edit::move_node(graph, *id, delta);
                }
                changed |= !state.selection.nodes().is_empty();
            }
            Some(Drag::Box { start, kept }) => {
                if let Some(pointer) = response.interact_pointer_pos() {
                    let area = Rect::from_two_pos(*start, at(pointer));
                    let mut selection = kept.clone();
                    for id in nodes_in(graph, &layouts, area) {
                        selection.add(id);
                    }
                    state.selection = selection;
                }
            }
            Some(Drag::Wire(_)) | None => {}
        }
    }
    if response.drag_stopped_by(PointerButton::Primary) {
        if let (Some(Drag::Wire(start)), Some(released)) =
            (state.drag.take(), response.interact_pointer_pos())
            && let Some(end) = pin_at(graph, &layouts, at(released))
        {
            changed |= edit::connect(graph, &start, &end);
        }
        state.drag = None;
    }
    if response.clicked()
        && let Some(pointer) = response.interact_pointer_pos()
    {
        let point = at(pointer);
        match (node_at(graph, &layouts, point), adding) {
            (Some(id), true) => state.selection.toggle(id),
            (Some(id), false) => state.selection = Selection::only(id),
            (None, true) => {}
            (None, false) => {
                state.selection = link_at(graph, &layouts, point)
                    .map_or_else(Selection::default, Selection::only_wire);
            }
        }
    }
    if response.secondary_clicked()
        && let Some(pointer) = response.interact_pointer_pos()
    {
        let point = at(pointer);
        state.menu = Some(if let Some(pin) = pin_at(graph, &layouts, point) {
            MenuTarget::Pin(pin)
        } else if let Some(id) = node_at(graph, &layouts, point) {
            if !state.selection.contains(id) {
                state.selection = Selection::only(id);
            }
            MenuTarget::Node(id)
        } else if let Some(link) = link_at(graph, &layouts, point) {
            state.selection = Selection::only_wire(link.clone());
            MenuTarget::Wire(link)
        } else {
            MenuTarget::Background(point)
        });
    }
    changed
}

/// Whether a click or a box adds to the selection instead of replacing it.
const fn adds_to_selection(modifiers: Modifiers) -> bool {
    modifiers.shift || modifiers.command
}

/// `Delete` deletes the selected wire, or the selected nodes but the
/// entry, which every graph needs, unless a text field is being typed in.
fn delete_selected_on_key(ui: &egui::Ui, graph: &mut Graph, state: &mut CanvasState) -> bool {
    let pressed =
        !ui.ctx().egui_wants_keyboard_input() && ui.input(|input| input.key_pressed(Key::Delete));
    if !pressed {
        return false;
    }
    let removed = match state.selection.wire() {
        Some(wire) => edit::remove_link(graph, &wire.clone()),
        None => edit::remove_nodes(graph, state.selection.nodes()),
    };
    state.selection.keep_existing(graph);
    removed
}

fn paint(
    ui: &egui::Ui,
    graph: &Graph,
    state: &CanvasState,
    response: &Response,
    rect: egui::Rect,
    mistakes: &NodeMistakes<'_>,
) {
    let painting = Painting {
        painter: ui.painter_at(rect),
        rect,
        view: state.view,
    };
    painting.paint_background();
    let layouts = graph_layout(graph);
    for link in &graph.links {
        painting.paint_wire(&layouts, link, state.selection.wire() == Some(link));
    }
    if let (Some(Drag::Wire(start)), Some(pointer)) = (&state.drag, response.interact_pointer_pos())
        && let Some(pin) = layouts
            .get(&start.node)
            .and_then(|layout| layout.pin(&start.pin))
    {
        painting.paint_pulled_wire(pin, pointer);
    }

    for node in &graph.nodes {
        if let Some(layout) = layouts.get(&node.id) {
            painting.paint_node(
                node,
                layout,
                state.selection.contains(node.id),
                mistakes.contains_key(&node.id),
            );
        }
    }
    if let (Some(Drag::Box { start, .. }), Some(pointer)) =
        (&state.drag, response.interact_pointer_pos())
    {
        painting.paint_selection_box(state.view.to_screen(rect.min, *start), pointer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shift_and_control_add_to_the_selection() {
        assert!(adds_to_selection(Modifiers::SHIFT));
        assert!(adds_to_selection(Modifiers::COMMAND));
        assert!(!adds_to_selection(Modifiers::NONE));
    }
}
