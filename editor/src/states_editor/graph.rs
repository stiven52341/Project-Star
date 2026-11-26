//! The graph of a state machine on a canvas:
//!
//! - states are boxes, the start one green, and “Any state” stands left
//!   of them for the transitions that leave any state;
//! - transitions are arrows, with their head halfway, beside the line
//!   between two states so the two ways do not cover each other;
//! - clicking selects a state or an arrow, and dragging a state moves it;
//! - the right button opens a menu: a new state where it was clicked, a
//!   new transition from a state (then clicking where it goes; `Esc`
//!   gives up), the start, deleting;
//! - dragging with the middle button, or scrolling on a touchpad, moves
//!   the view; the wheel and pinching zoom; `Delete` deletes what is
//!   selected.

use eframe::egui::{
    self, Align2, Color32, FontId, Key, PointerButton, Pos2, Rect, Response, Sense, Shape, Stroke,
    Vec2, pos2, vec2,
};
use star_engine::star_scene::{AnimationState, AnimationStates, Motion, StateTransition};

use super::edits;
use crate::canvas_view::{CanvasView, scroll_view};
use crate::texts;

/// The size of a state's box, in graph points.
const STATE_SIZE: Vec2 = vec2(150.0, 44.0);
/// How far left of the leftmost state “Any state” stands.
const ANY_GAP: f32 = 90.0;
/// How far beside the line between two boxes their arrows go.
const ARROW_OFFSET: f32 = 7.0;
/// The length of an arrow's head.
const ARROW_HEAD: f32 = 9.0;
/// How near, in screen points, a click must be to an arrow to pick it.
const PICK_DISTANCE: f32 = 6.0;
const ROUNDING: f32 = 6.0;
const NAME_SIZE: f32 = 14.0;
const MOTION_SIZE: f32 = 11.0;
/// How far below a box's middle its two lines of text are.
const LINE_SHIFT: f32 = 8.0;
const LINE_WIDTH: f32 = 2.0;

const BACKGROUND: Color32 = Color32::from_rgb(32, 33, 38);
const STATE_FILL: Color32 = Color32::from_rgb(62, 65, 76);
const START_FILL: Color32 = Color32::from_rgb(48, 112, 72);
const ANY_FILL: Color32 = Color32::from_rgb(112, 84, 44);
const TEXT: Color32 = Color32::WHITE;
const MOTION_TEXT: Color32 = Color32::from_rgb(190, 192, 200);
const ARROW: Color32 = Color32::from_rgb(196, 198, 208);
const SELECTED: Color32 = Color32::from_rgb(242, 200, 80);

/// What is selected on the graph, for the inspector.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum Selected {
    #[default]
    Nothing,
    State(usize),
    Transition(usize),
}

/// What the canvas remembers between frames.
#[derive(Default)]
pub(super) struct GraphState {
    view: CanvasView,
    /// The state being dragged.
    dragging: Option<usize>,
    /// A transition being made, to the state clicked next.
    linking: Option<LinkFrom>,
    menu: Option<MenuTarget>,
}

/// Where a transition being made comes from.
#[derive(Debug, Clone)]
enum LinkFrom {
    Any,
    State(String),
}

impl LinkFrom {
    /// The state it leaves, or `None` for any.
    fn state(&self) -> Option<&str> {
        match self {
            Self::Any => None,
            Self::State(name) => Some(name),
        }
    }
}

/// What the open menu of the right button is about.
#[derive(Debug, Clone, Copy)]
enum MenuTarget {
    /// The background, at this point of the graph.
    Background(Pos2),
    State(usize),
    Any,
    Transition(usize),
}

impl GraphState {
    /// Whether a transition is being made, waiting for the state it goes
    /// to.
    pub(super) const fn is_linking(&self) -> bool {
        self.linking.is_some()
    }
}

/// Draws `machine` filling the space left in `ui` and applies what the
/// mouse and keyboard do to it. New states play `animation`. Returns
/// whether the machine changed.
pub(super) fn show_graph(
    ui: &mut egui::Ui,
    machine: &mut AnimationStates,
    selected: &mut Selected,
    state: &mut GraphState,
    animation: &str,
) -> bool {
    let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
    let origin = rect.min;
    if response.dragged_by(PointerButton::Middle) {
        state.view.pan_by(response.drag_delta());
    }
    scroll_view(ui, &mut state.view, &response, origin);
    if ui.input(|input| input.key_pressed(Key::Escape)) {
        state.linking = None;
    }
    let pressed = ui.input(|input| input.pointer.press_origin());
    let mut changed = handle_pointer(machine, selected, state, &response, (origin, pressed));
    response.context_menu(|ui| {
        if let Some(target) = state.menu {
            changed |= show_menu(ui, machine, selected, state, target, animation);
        }
    });
    changed |= delete_on_key(ui, machine, selected);
    paint(ui, machine, *selected, state, &response, rect);
    changed
}

/// Selects, drags states, makes transitions and opens menus with the
/// mouse, on a canvas whose top-left is `origin`; `pressed` is where the
/// main button went down. Returns whether the machine changed.
fn handle_pointer(
    machine: &mut AnimationStates,
    selected: &mut Selected,
    state: &mut GraphState,
    response: &Response,
    (origin, pressed): (Pos2, Option<Pos2>),
) -> bool {
    let view = state.view;
    let at = |screen: Pos2| view.to_graph(origin, screen);
    let pick = PICK_DISTANCE / view.zoom();
    let mut changed = false;
    if response.drag_started_by(PointerButton::Primary)
        && let Some(pressed) = pressed
    {
        state.dragging = state_at(machine, at(pressed));
        if let Some(index) = state.dragging {
            *selected = Selected::State(index);
        }
    }
    if response.dragged_by(PointerButton::Primary)
        && let Some(moved) = state
            .dragging
            .and_then(|index| machine.states.get_mut(index))
    {
        let delta = response.drag_delta() / view.zoom();
        moved.position[0] += delta.x;
        moved.position[1] += delta.y;
        changed = true;
    }
    if response.drag_stopped() {
        state.dragging = None;
    }
    if response.clicked()
        && let Some(pointer) = response.interact_pointer_pos()
    {
        changed |= click(machine, selected, state, at(pointer), pick);
    }
    if response.secondary_clicked()
        && let Some(pointer) = response.interact_pointer_pos()
    {
        let point = at(pointer);
        state.linking = None;
        state.menu = Some(if let Some(index) = state_at(machine, point) {
            *selected = Selected::State(index);
            MenuTarget::State(index)
        } else if any_rect(machine).contains(point) {
            MenuTarget::Any
        } else if let Some(index) = transition_at(machine, point, pick) {
            *selected = Selected::Transition(index);
            MenuTarget::Transition(index)
        } else {
            MenuTarget::Background(point)
        });
    }
    changed
}

/// A click at `point`: ends the transition being made on the state
/// clicked, or else selects what is there. Returns whether the machine
/// changed.
fn click(
    machine: &mut AnimationStates,
    selected: &mut Selected,
    state: &mut GraphState,
    point: Pos2,
    pick: f32,
) -> bool {
    let target = state_at(machine, point);
    if let Some(from) = state.linking.take() {
        let made = target
            .and_then(|index| machine.states.get(index))
            .map(|to| to.name.clone())
            .and_then(|to| edits::add_transition(machine, from.state(), &to));
        if let Some(index) = made {
            *selected = Selected::Transition(index);
        }
        return made.is_some();
    }
    *selected = match (target, transition_at(machine, point, pick)) {
        (Some(index), _) => Selected::State(index),
        (None, Some(index)) => Selected::Transition(index),
        (None, None) => Selected::Nothing,
    };
    false
}

/// The entries of the menu of the right button about `target`. Returns
/// whether the machine changed.
fn show_menu(
    ui: &mut egui::Ui,
    machine: &mut AnimationStates,
    selected: &mut Selected,
    state: &mut GraphState,
    target: MenuTarget,
    animation: &str,
) -> bool {
    let texts = texts::current();
    let mut changed = false;
    match target {
        MenuTarget::Background(point) => {
            if ui.button(texts.new_state()).clicked() {
                let corner = point - STATE_SIZE / 2.0;
                let index = edits::add_state(machine, [corner.x, corner.y], animation);
                *selected = Selected::State(index);
                changed = true;
                ui.close();
            }
        }
        MenuTarget::State(index) => {
            let Some(clicked) = machine.states.get(index) else {
                return false;
            };
            let name = clicked.name.clone();
            if ui.button(texts.new_transition()).clicked() {
                state.linking = Some(LinkFrom::State(name.clone()));
                ui.close();
            }
            if machine.start != name && ui.button(texts.set_start_state()).clicked() {
                machine.start = name;
                changed = true;
                ui.close();
            }
            if machine.states.len() > 1 && ui.button(texts.delete_state()).clicked() {
                changed = edits::delete_state(machine, index);
                *selected = Selected::Nothing;
                ui.close();
            }
        }
        MenuTarget::Any => {
            if ui.button(texts.new_transition_from_any()).clicked() {
                state.linking = Some(LinkFrom::Any);
                ui.close();
            }
        }
        MenuTarget::Transition(index) => {
            if ui.button(texts.delete_transition()).clicked() {
                changed = delete(machine, Selected::Transition(index));
                *selected = Selected::Nothing;
                ui.close();
            }
        }
    }
    changed
}

/// `Delete` deletes the state or transition selected, unless a text field
/// is being typed in.
fn delete_on_key(ui: &egui::Ui, machine: &mut AnimationStates, selected: &mut Selected) -> bool {
    let pressed =
        !ui.ctx().egui_wants_keyboard_input() && ui.input(|input| input.key_pressed(Key::Delete));
    if !pressed || !delete(machine, *selected) {
        return false;
    }
    *selected = Selected::Nothing;
    true
}

/// Deletes what `selected` is. Returns whether it did.
fn delete(machine: &mut AnimationStates, selected: Selected) -> bool {
    match selected {
        Selected::State(index) => edits::delete_state(machine, index),
        Selected::Transition(index) if index < machine.transitions.len() => {
            machine.transitions.remove(index);
            true
        }
        Selected::Nothing | Selected::Transition(_) => false,
    }
}

/// The box of `state`, in the graph.
fn state_rect(state: &AnimationState) -> Rect {
    Rect::from_min_size(pos2(state.position[0], state.position[1]), STATE_SIZE)
}

/// The box of “Any state”: left of the leftmost state, at the height of
/// the highest.
fn any_rect(machine: &AnimationStates) -> Rect {
    let (left, top) = machine
        .states
        .iter()
        .fold(None, |corner: Option<(f32, f32)>, state| {
            let [x, y] = state.position;
            Some(corner.map_or((x, y), |(left, top)| (left.min(x), top.min(y))))
        })
        .unwrap_or_default();
    Rect::from_min_size(pos2(left - ANY_GAP - STATE_SIZE.x, top), STATE_SIZE)
}

/// The state whose box holds `point`, the one drawn on top if several do.
fn state_at(machine: &AnimationStates, point: Pos2) -> Option<usize> {
    machine
        .states
        .iter()
        .rposition(|state| state_rect(state).contains(point))
}

/// Where the arrow of `transition` starts and ends, in the graph.
fn arrow_ends(machine: &AnimationStates, transition: &StateTransition) -> Option<(Pos2, Pos2)> {
    let from = match &transition.from {
        Some(name) => state_rect(machine.state(name)?).center(),
        None => any_rect(machine).center(),
    };
    let to = state_rect(machine.state(&transition.to)?).center();
    let side = (to - from).normalized().rot90() * ARROW_OFFSET;
    Some((from + side, to + side))
}

/// The transition whose arrow passes within `distance` of `point`.
fn transition_at(machine: &AnimationStates, point: Pos2, distance: f32) -> Option<usize> {
    machine.transitions.iter().position(|transition| {
        arrow_ends(machine, transition)
            .is_some_and(|(from, to)| distance_to_segment(point, from, to) <= distance)
    })
}

fn distance_to_segment(point: Pos2, start: Pos2, end: Pos2) -> f32 {
    let line = end - start;
    let length = line.length_sq();
    if length <= f32::EPSILON {
        return point.distance(start);
    }
    let along = ((point - start).dot(line) / length).clamp(0.0, 1.0);
    point.distance(start + line * along)
}

fn paint(
    ui: &egui::Ui,
    machine: &AnimationStates,
    selected: Selected,
    state: &GraphState,
    response: &Response,
    rect: Rect,
) {
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, BACKGROUND);
    let view = state.view;
    let screen = |point: Pos2| view.to_screen(rect.min, point);
    for (index, transition) in machine.transitions.iter().enumerate() {
        if let Some((from, to)) = arrow_ends(machine, transition) {
            let color = if selected == Selected::Transition(index) {
                SELECTED
            } else {
                ARROW
            };
            paint_arrow(&painter, screen(from), screen(to), color, view.zoom());
        }
    }
    if let (Some(from), Some(pointer)) = (&state.linking, response.hover_pos()) {
        let start = from
            .state()
            .and_then(|name| machine.state(name))
            .map_or_else(
                || any_rect(machine).center(),
                |from| state_rect(from).center(),
            );
        paint_arrow(&painter, screen(start), pointer, SELECTED, view.zoom());
    }
    let box_on_screen = |area: Rect| view.rect_to_screen(rect.min, area);
    paint_box(
        &painter,
        box_on_screen(any_rect(machine)),
        ANY_FILL,
        false,
        (texts::current().any_state(), ""),
        view.zoom(),
    );
    for (index, shown) in machine.states.iter().enumerate() {
        let fill = if shown.name == machine.start {
            START_FILL
        } else {
            STATE_FILL
        };
        paint_box(
            &painter,
            box_on_screen(state_rect(shown)),
            fill,
            selected == Selected::State(index),
            (&shown.name, &motion_label(&shown.motion)),
            view.zoom(),
        );
    }
}

/// What a box says a state plays.
fn motion_label(motion: &Motion) -> String {
    match motion {
        Motion::Animation(animation) => animation.clone(),
        Motion::Blend { value, points } => texts::current().blend_label(value, points.len()),
    }
}

fn paint_box(
    painter: &egui::Painter,
    area: Rect,
    fill: Color32,
    selected: bool,
    (name, motion): (&str, &str),
    zoom: f32,
) {
    painter.rect_filled(area, ROUNDING * zoom, fill);
    if selected {
        painter.rect_stroke(
            area,
            ROUNDING * zoom,
            Stroke::new(LINE_WIDTH, SELECTED),
            egui::StrokeKind::Outside,
        );
    }
    let shift = if motion.is_empty() {
        0.0
    } else {
        LINE_SHIFT * zoom
    };
    painter.text(
        area.center() - vec2(0.0, shift),
        Align2::CENTER_CENTER,
        name,
        FontId::proportional(NAME_SIZE * zoom),
        TEXT,
    );
    if !motion.is_empty() {
        painter.text(
            area.center() + vec2(0.0, shift),
            Align2::CENTER_CENTER,
            motion,
            FontId::proportional(MOTION_SIZE * zoom),
            MOTION_TEXT,
        );
    }
}

/// A line from `from` to `to` with its head halfway, pointing to `to`.
fn paint_arrow(painter: &egui::Painter, from: Pos2, to: Pos2, color: Color32, zoom: f32) {
    painter.line_segment([from, to], Stroke::new(LINE_WIDTH, color));
    let direction = (to - from).normalized();
    if direction == Vec2::ZERO {
        return;
    }
    let head = ARROW_HEAD * zoom;
    let tip = from + (to - from) / 2.0 + direction * head / 2.0;
    let back = tip - direction * head;
    let side = direction.rot90() * head / 2.0;
    painter.add(Shape::convex_polygon(
        vec![tip, back + side, back - side],
        color,
        Stroke::NONE,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn machine() -> AnimationStates {
        AnimationStates::from_text(
            "format = 1\nstart = \"idle\"\n\
             [[state]]\nname = \"idle\"\nanimation = \"idle\"\nposition = [0.0, 0.0]\n\
             [[state]]\nname = \"run\"\nanimation = \"run\"\nposition = [300.0, 0.0]\n\
             [[transition]]\nfrom = \"idle\"\nto = \"run\"\n\
             [[transition]]\nfrom = \"run\"\nto = \"idle\"\n\
             [[transition]]\nto = \"run\"\n",
        )
        .unwrap()
    }

    #[test]
    fn states_are_picked_by_their_box() {
        let machine = machine();

        assert_eq!(state_at(&machine, pos2(10.0, 10.0)), Some(0));
        assert_eq!(state_at(&machine, pos2(310.0, 10.0)), Some(1));
        assert_eq!(state_at(&machine, pos2(200.0, 10.0)), None);
    }

    #[test]
    fn any_state_stands_left_of_the_states() {
        let machine = machine();

        let any = any_rect(&machine);

        assert!(any.max.x < 0.0);
        assert!((any.min.y - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn the_two_ways_between_states_are_picked_apart() {
        let machine = machine();
        let middle_y = STATE_SIZE.y / 2.0;

        // Going right, the arrow is above the line between the middles;
        // coming back, below it.
        assert_eq!(
            transition_at(&machine, pos2(225.0, middle_y - ARROW_OFFSET), 2.0),
            Some(0)
        );
        assert_eq!(
            transition_at(&machine, pos2(225.0, middle_y + ARROW_OFFSET), 2.0),
            Some(1)
        );
    }

    #[test]
    fn segments_measure_to_their_nearest_point() {
        let start = pos2(0.0, 0.0);
        let end = pos2(10.0, 0.0);

        assert!((distance_to_segment(pos2(5.0, 3.0), start, end) - 3.0).abs() < 1e-5);
        assert!((distance_to_segment(pos2(-4.0, 3.0), start, end) - 5.0).abs() < 1e-5);
    }
}
