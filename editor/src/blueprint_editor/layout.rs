//! Where each node and pin of a graph is drawn, in graph units: the
//! positions the blueprint stores, before the view's pan and zoom.

use std::collections::HashMap;

use eframe::egui::{Pos2, Rect, pos2, vec2};
use star_blueprint::{Direction, Graph, Node, NodeId, Pin};

/// Width of every node.
pub(super) const NODE_WIDTH: f32 = 180.0;
/// Height of the band with the node's title.
pub(super) const HEADER_HEIGHT: f32 = 26.0;
/// Height of each row of pins.
pub(super) const ROW_HEIGHT: f32 = 22.0;
/// Space below the last row of pins.
const BOTTOM_PADDING: f32 = 6.0;

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
    fn every_node_gets_a_layout() {
        let graph = graph_with(NodeKind::Return);

        let layouts = graph_layout(&graph);

        assert!(layouts.contains_key(&NodeId(1)));
        assert_eq!(layouts.len(), 1);
    }
}
