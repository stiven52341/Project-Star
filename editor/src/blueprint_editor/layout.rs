//! Where each node and pin of a graph is drawn, in graph units: the
//! positions the blueprint stores, before the view's pan and zoom.

use std::collections::HashMap;

use eframe::egui::{Pos2, Rect, pos2, vec2};
use star_blueprint::{Direction, Graph, Node, NodeId, Pin, PinRef};

/// Width of every node.
pub(super) const NODE_WIDTH: f32 = 180.0;
/// Height of the band with the node's title.
pub(super) const HEADER_HEIGHT: f32 = 26.0;
/// Height of each row of pins.
pub(super) const ROW_HEIGHT: f32 = 22.0;
/// Space below the last row of pins.
const BOTTOM_PADDING: f32 = 6.0;
/// How far from a pin the pointer still grabs it.
const PIN_GRAB_RADIUS: f32 = 10.0;

/// A pin and where its wires end.
pub(super) struct PlacedPin {
    pub(super) pin: Pin,
    pub(super) position: Pos2,
}

/// The box of a node and its pins: inputs down its left side, outputs down
/// its right side, in the order of [`Node::pins`].
pub(super) struct NodeLayout {
    pub(super) rect: Rect,
    pub(super) pins: Vec<PlacedPin>,
}

impl NodeLayout {
    pub(super) fn new(node: &Node, graph: &Graph) -> Self {
        let origin = pos2(node.position[0], node.position[1]);
        let first_row = HEADER_HEIGHT + ROW_HEIGHT / 2.0;
        let mut input_row = first_row;
        let mut output_row = first_row;
        let pins = node
            .pins(graph)
            .into_iter()
            .map(|pin| {
                let (x, row) = match pin.direction {
                    Direction::Input => (0.0, &mut input_row),
                    Direction::Output => (NODE_WIDTH, &mut output_row),
                };
                let position = origin + vec2(x, *row);
                *row += ROW_HEIGHT;
                PlacedPin { pin, position }
            })
            .collect();
        let rows_end = input_row.max(output_row) - ROW_HEIGHT / 2.0;
        Self {
            rect: Rect::from_min_size(origin, vec2(NODE_WIDTH, rows_end + BOTTOM_PADDING)),
            pins,
        }
    }

    /// The pin called `name`.
    pub(super) fn pin(&self, name: &str) -> Option<&PlacedPin> {
        self.pins.iter().find(|placed| placed.pin.name == name)
    }
}

/// The layout of every node of `graph`.
pub(super) fn graph_layout(graph: &Graph) -> HashMap<NodeId, NodeLayout> {
    graph
        .nodes
        .iter()
        .map(|node| (node.id, NodeLayout::new(node, graph)))
        .collect()
}

/// The pin of `graph` closest to `point`, if one is close enough to grab.
pub(super) fn pin_at(
    graph: &Graph,
    layouts: &HashMap<NodeId, NodeLayout>,
    point: Pos2,
) -> Option<PinRef> {
    graph
        .nodes
        .iter()
        .filter_map(|node| Some((node.id, layouts.get(&node.id)?)))
        .flat_map(|(id, layout)| layout.pins.iter().map(move |placed| (id, placed)))
        .map(|(id, placed)| (id, placed, placed.position.distance(point)))
        .filter(|(_, _, distance)| *distance <= PIN_GRAB_RADIUS)
        .min_by(|first, second| first.2.total_cmp(&second.2))
        .map(|(id, placed, _)| PinRef::new(id, &placed.pin.name))
}

/// The node of `graph` under `point`: the one drawn last, on top, if
/// several are.
pub(super) fn node_at(
    graph: &Graph,
    layouts: &HashMap<NodeId, NodeLayout>,
    point: Pos2,
) -> Option<NodeId> {
    graph
        .nodes
        .iter()
        .rev()
        .find(|node| {
            layouts
                .get(&node.id)
                .is_some_and(|layout| layout.rect.contains(point))
        })
        .map(|node| node.id)
}

#[cfg(test)]
mod tests {
    use star_blueprint::{BRANCH_FALSE, BRANCH_TRUE, CONDITION, EXECUTION_INPUT, Graph, NodeKind};

    use super::*;

    fn graph_with(kind: NodeKind) -> Graph {
        Graph {
            name: "update".to_owned(),
            parameters: Vec::new(),
            returns: None,
            is_override: false,
            private: false,
            nodes: vec![Node {
                id: NodeId(1),
                position: [100.0, 50.0],
                kind,
            }],
            links: Vec::new(),
        }
    }

    #[test]
    fn inputs_go_left_and_outputs_right_row_by_row() {
        let graph = graph_with(NodeKind::Branch);

        let layout = NodeLayout::new(&graph.nodes[0], &graph);

        let position = |name| layout.pin(name).expect("a pin").position;
        let first_row = 50.0 + HEADER_HEIGHT + ROW_HEIGHT / 2.0;
        assert_eq!(position(EXECUTION_INPUT), pos2(100.0, first_row));
        assert_eq!(position(CONDITION), pos2(100.0, first_row + ROW_HEIGHT));
        assert_eq!(position(BRANCH_TRUE), pos2(280.0, first_row));
        assert_eq!(position(BRANCH_FALSE), pos2(280.0, first_row + ROW_HEIGHT));
    }

    #[test]
    fn nodes_are_as_tall_as_their_longest_side() {
        let graph = graph_with(NodeKind::Branch);

        let layout = NodeLayout::new(&graph.nodes[0], &graph);

        assert_eq!(layout.rect.min, pos2(100.0, 50.0));
        assert_eq!(
            layout.rect.height(),
            HEADER_HEIGHT + 2.0 * ROW_HEIGHT + BOTTOM_PADDING
        );
    }

    #[test]
    fn pins_are_grabbed_near_them() {
        let graph = graph_with(NodeKind::Branch);
        let layouts = graph_layout(&graph);
        let condition = layouts[&NodeId(1)].pin(CONDITION).expect("a pin").position;

        let grabbed = pin_at(&graph, &layouts, condition + vec2(3.0, 4.0));

        assert_eq!(grabbed, Some(PinRef::new(NodeId(1), CONDITION)));
        assert_eq!(pin_at(&graph, &layouts, condition + vec2(30.0, 0.0)), None);
    }

    #[test]
    fn the_node_on_top_is_the_one_found() {
        let mut graph = graph_with(NodeKind::Branch);
        graph.nodes.push(Node {
            id: NodeId(2),
            position: [120.0, 60.0],
            kind: NodeKind::Return,
        });
        let layouts = graph_layout(&graph);

        assert_eq!(
            node_at(&graph, &layouts, pos2(130.0, 70.0)),
            Some(NodeId(2))
        );
        assert_eq!(
            node_at(&graph, &layouts, pos2(105.0, 55.0)),
            Some(NodeId(1))
        );
        assert_eq!(node_at(&graph, &layouts, pos2(0.0, 0.0)), None);
    }

    #[test]
    fn every_node_gets_a_layout() {
        let graph = graph_with(NodeKind::Return);

        let layouts = graph_layout(&graph);

        assert!(layouts.contains_key(&NodeId(1)));
        assert_eq!(layouts.len(), 1);
    }
}
