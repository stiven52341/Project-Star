//! The canvas of a graph and what the mouse does on it:
//!
//! - dragging the background moves the view, and the wheel zooms;
//! - clicking a node selects it, and dragging it moves it;
//! - dragging from a pin to another one joins them with a wire;
//! - the right button opens a menu to add nodes, delete them or remove
//!   wires; `Delete` deletes the selected node.
//!
//! Nodes with mistakes found by «Compilar» have a red border, and the
//! pointer over them shows the mistakes.

use std::collections::HashMap;

use eframe::egui::{self, Key, Pos2, Response, Sense};
use star_blueprint::{Graph, NodeId, NodeKind, PinRef};

use super::edit;
use super::layout::{graph_layout, node_at, pin_at};
use super::menus::{MenuTarget, show_menu};
use super::names::BlueprintNames;
use super::painting::Painting;
use super::view::CanvasView;

/// How much one point of mouse wheel zooms.
const ZOOM_PER_SCROLL: f32 = 0.002;

/// What the canvas of one blueprint remembers between frames.
#[derive(Default)]
pub(super) struct CanvasState {
    pub(super) view: CanvasView,
    /// The node whose properties the inspector shows.
    pub(super) selected: Option<NodeId>,
    drag: Option<Drag>,
    /// What the open menu of the right button is about.
    menu: Option<MenuTarget>,
    /// A node to bring to the middle of the canvas, once its size is known.
    focus: Option<NodeId>,
}

/// The mistakes of each node of the graph shown.
pub(super) type NodeMistakes<'problems> = HashMap<NodeId, Vec<&'problems str>>;

/// What a drag that is going on does.
enum Drag {
    View,
    Node(NodeId),
    Wire(PinRef),
}

impl CanvasState {
    /// Forgets what was about the graph shown, to show another one.
    pub(super) fn show_other_graph(&mut self) {
        *self = Self::default();
    }

    /// Selects the node `id` and brings it to the middle of the canvas.
    pub(super) fn focus_on(&mut self, id: NodeId) {
        self.selected = Some(id);
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
    if state.selected.is_some_and(|id| graph.node(id).is_none()) {
        state.selected = None;
    }
    let selected_before = state.selected;
    let mut changed = handle_pointer(ui, graph, state, &response, rect.min);
    response.context_menu(|ui| {
        if let Some(target) = &state.menu {
            changed |= show_menu(ui, graph, target, names, &mut state.selected);
        }
    });
    changed |= delete_selected_on_key(ui, graph, state);
    if state.selected != selected_before {
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

/// Moves the view, nodes and wires with the mouse. Returns whether the
/// graph changed.
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
    let mut changed = false;
    if response.drag_started()
        && let Some(pressed) = ui.input(|input| input.pointer.press_origin())
    {
        let point = at(pressed);
        state.drag = Some(if let Some(pin) = pin_at(graph, &layouts, point) {
            Drag::Wire(pin)
        } else if let Some(id) = node_at(graph, &layouts, point) {
            state.selected = Some(id);
            Drag::Node(id)
        } else {
            Drag::View
        });
    }
    if response.dragged() {
        match &state.drag {
            Some(Drag::View) => state.view.pan_by(response.drag_delta()),
            Some(Drag::Node(id)) => {
                edit::move_node(graph, *id, response.drag_delta() / state.view.zoom());
                changed = true;
            }
            Some(Drag::Wire(_)) | None => {}
        }
    }
    if response.drag_stopped() {
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
        state.selected = node_at(graph, &layouts, at(pointer));
    }
    if response.secondary_clicked()
        && let Some(pointer) = response.interact_pointer_pos()
    {
        let point = at(pointer);
        state.menu = Some(if let Some(pin) = pin_at(graph, &layouts, point) {
            MenuTarget::Pin(pin)
        } else if let Some(id) = node_at(graph, &layouts, point) {
            state.selected = Some(id);
            MenuTarget::Node(id)
        } else {
            MenuTarget::Background(point)
        });
    }
    if let Some(pointer) = response.hover_pos() {
        let scroll = ui.input(|input| input.smooth_scroll_delta.y);
        if scroll.abs() > f32::EPSILON {
            state
                .view
                .zoom_around(origin, pointer, (scroll * ZOOM_PER_SCROLL).exp());
        }
    }
    changed
}

/// `Delete` deletes the selected node, unless a text field is being typed
/// in or it is the entry, which every graph needs.
fn delete_selected_on_key(ui: &egui::Ui, graph: &mut Graph, state: &mut CanvasState) -> bool {
    let Some(id) = state.selected else {
        return false;
    };
    let is_entry = graph
        .node(id)
        .is_some_and(|node| matches!(node.kind, NodeKind::Entry));
    let pressed =
        !ui.ctx().egui_wants_keyboard_input() && ui.input(|input| input.key_pressed(Key::Delete));
    if !pressed || is_entry {
        return false;
    }
    edit::remove_node(graph, id);
    state.selected = None;
    true
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
        painting.paint_wire(&layouts, link);
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
                state.selected == Some(node.id),
                mistakes.contains_key(&node.id),
            );
        }
    }
}
