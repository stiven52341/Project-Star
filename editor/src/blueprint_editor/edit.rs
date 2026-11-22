//! Changes made to a graph from the editor.
//!
//! They keep the wires the editor can draw: each one joins an output to an
//! input of the same kind, on two different nodes, and a pin that takes a
//! single wire (every input, and execution outputs) keeps the newest one.
//! Everything else that can be wrong in a graph is left to
//! [`validate`](star_blueprint::validate), as with any half-built graph.

use eframe::egui::{Pos2, Vec2};
use star_blueprint::{Direction, Graph, Link, Node, NodeId, NodeKind, Pin, PinKind, PinRef};

/// Adds a node of `kind` at `position`; returns its identifier.
pub(super) fn add_node(graph: &mut Graph, kind: NodeKind, position: Pos2) -> NodeId {
    let id = NodeId(
        graph
            .nodes
            .iter()
            .map(|node| node.id.0 + 1)
            .max()
            .unwrap_or(1),
    );
    graph.nodes.push(Node {
        id,
        position: [position.x, position.y],
        kind,
    });
    id
}

/// Removes the node `id` and its wires.
pub(super) fn remove_node(graph: &mut Graph, id: NodeId) {
    graph.nodes.retain(|node| node.id != id);
    graph
        .links
        .retain(|link| link.from.node != id && link.to.node != id);
}

/// Moves the node `id` by `delta`, in graph units.
pub(super) fn move_node(graph: &mut Graph, id: NodeId, delta: Vec2) {
    if let Some(node) = graph.nodes.iter_mut().find(|node| node.id == id) {
        node.position[0] += delta.x;
        node.position[1] += delta.y;
    }
}

/// Joins two pins with a wire, whichever of them is the output. Returns
/// whether they could be joined.
pub(super) fn connect(graph: &mut Graph, first: &PinRef, second: &PinRef) -> bool {
    let (Some(first_pin), Some(second_pin)) = (pin(graph, first), pin(graph, second)) else {
        return false;
    };
    if first.node == second.node || first_pin.kind != second_pin.kind {
        return false;
    }
    let (from, to) = match (first_pin.direction, second_pin.direction) {
        (Direction::Output, Direction::Input) => (first, second),
        (Direction::Input, Direction::Output) => (second, first),
        _ => return false,
    };
    let single_output = first_pin.kind == PinKind::Execution;
    graph
        .links
        .retain(|link| link.to != *to && !(single_output && link.from == *from));
    graph.links.push(Link {
        from: from.clone(),
        to: to.clone(),
    });
    true
}

/// Removes every wire of `pin`.
pub(super) fn disconnect(graph: &mut Graph, pin: &PinRef) {
    graph
        .links
        .retain(|link| link.from != *pin && link.to != *pin);
}

/// Whether `pin` has any wire.
pub(super) fn is_connected(graph: &Graph, pin: &PinRef) -> bool {
    graph
        .links
        .iter()
        .any(|link| link.from == *pin || link.to == *pin)
}

/// Gives the wires of the pin `old` of the node `id` to its pin `new`, for a
/// pin that was renamed.
pub(super) fn rename_pin(graph: &mut Graph, id: NodeId, old: &str, new: &str) {
    for end in graph
        .links
        .iter_mut()
        .flat_map(|link| [&mut link.from, &mut link.to])
    {
        if end.node == id && end.pin == old {
            new.clone_into(&mut end.pin);
        }
    }
}

/// Removes the wires whose pins are gone, after a node changed the pins it
/// has (an operator that takes one value now, a call without a result).
pub(super) fn remove_broken_links(graph: &mut Graph) {
    let fits =
        |end: &PinRef, direction| pin(graph, end).is_some_and(|pin| pin.direction == direction);
    let kept = graph
        .links
        .iter()
        .filter(|link| fits(&link.from, Direction::Output) && fits(&link.to, Direction::Input))
        .cloned()
        .collect();
    graph.links = kept;
}

fn pin(graph: &Graph, end: &PinRef) -> Option<Pin> {
    graph.node(end.node)?.pin(graph, &end.pin)
}

#[cfg(test)]
mod tests {
    use eframe::egui::{pos2, vec2};
    use star_blueprint::{
        BRANCH_TRUE, CONDITION, EXECUTION_INPUT, EXECUTION_OUTPUT, Operator, RESULT, VALUE,
    };

    use super::*;

    /// An entry (1), a branch (2), a `!` (3) and a return (4).
    fn graph() -> Graph {
        let mut graph = Graph {
            name: "check".to_owned(),
            parameters: vec![star_blueprint::Parameter {
                name: "fast".to_owned(),
                ty: "bool".to_owned(),
            }],
            returns: None,
            is_override: false,
            private: false,
            nodes: Vec::new(),
            links: Vec::new(),
        };
        add_node(&mut graph, NodeKind::Entry, Pos2::ZERO);
        add_node(&mut graph, NodeKind::Branch, Pos2::ZERO);
        let not = NodeKind::Operator {
            operator: Operator::Not,
        };
        add_node(&mut graph, not, Pos2::ZERO);
        add_node(&mut graph, NodeKind::Return, Pos2::ZERO);
        graph
    }

    fn pin(node: u32, name: &str) -> PinRef {
        PinRef::new(NodeId(node), name)
    }

    fn links(graph: &Graph) -> Vec<(u32, &str, u32, &str)> {
        graph
            .links
            .iter()
            .map(|link| {
                (
                    link.from.node.0,
                    link.from.pin.as_str(),
                    link.to.node.0,
                    link.to.pin.as_str(),
                )
            })
            .collect()
    }

    #[test]
    fn new_nodes_get_the_next_identifier() {
        let mut graph = graph();

        let id = add_node(&mut graph, NodeKind::Return, pos2(10.0, 20.0));

        assert_eq!(id, NodeId(5));
        assert_eq!(graph.node(id).map(|node| node.position), Some([10.0, 20.0]));
    }

    #[test]
    fn wires_go_from_output_to_input_whichever_pin_comes_first() {
        let mut graph = graph();

        assert!(connect(
            &mut graph,
            &pin(2, EXECUTION_INPUT),
            &pin(1, EXECUTION_OUTPUT)
        ));

        assert_eq!(links(&graph), [(1, EXECUTION_OUTPUT, 2, EXECUTION_INPUT)]);
    }

    #[test]
    fn pins_of_different_kinds_or_sides_are_not_joined() {
        let mut graph = graph();

        assert!(!connect(
            &mut graph,
            &pin(1, "fast"),
            &pin(2, EXECUTION_INPUT)
        ));
        assert!(!connect(
            &mut graph,
            &pin(2, CONDITION),
            &pin(4, EXECUTION_INPUT)
        ));
        assert!(!connect(&mut graph, &pin(3, VALUE), &pin(3, RESULT)));
        assert!(!connect(&mut graph, &pin(1, "missing"), &pin(2, CONDITION)));
        assert_eq!(graph.links, []);
    }

    #[test]
    fn inputs_keep_only_their_newest_wire() {
        let mut graph = graph();
        connect(&mut graph, &pin(1, "fast"), &pin(2, CONDITION));

        connect(&mut graph, &pin(3, RESULT), &pin(2, CONDITION));

        assert_eq!(links(&graph), [(3, RESULT, 2, CONDITION)]);
    }

    #[test]
    fn execution_outputs_keep_only_their_newest_wire() {
        let mut graph = graph();
        connect(
            &mut graph,
            &pin(1, EXECUTION_OUTPUT),
            &pin(2, EXECUTION_INPUT),
        );

        connect(
            &mut graph,
            &pin(1, EXECUTION_OUTPUT),
            &pin(4, EXECUTION_INPUT),
        );

        assert_eq!(links(&graph), [(1, EXECUTION_OUTPUT, 4, EXECUTION_INPUT)]);
    }

    #[test]
    fn data_outputs_feed_many_inputs() {
        let mut graph = graph();

        connect(&mut graph, &pin(1, "fast"), &pin(2, CONDITION));
        connect(&mut graph, &pin(1, "fast"), &pin(3, VALUE));

        assert_eq!(graph.links.len(), 2);
    }

    #[test]
    fn removed_nodes_take_their_wires() {
        let mut graph = graph();
        connect(
            &mut graph,
            &pin(1, EXECUTION_OUTPUT),
            &pin(2, EXECUTION_INPUT),
        );
        connect(&mut graph, &pin(2, BRANCH_TRUE), &pin(4, EXECUTION_INPUT));

        remove_node(&mut graph, NodeId(2));

        assert_eq!(graph.node(NodeId(2)), None);
        assert_eq!(graph.links, []);
    }

    #[test]
    fn disconnected_pins_lose_every_wire() {
        let mut graph = graph();
        connect(&mut graph, &pin(1, "fast"), &pin(2, CONDITION));
        connect(&mut graph, &pin(1, "fast"), &pin(3, VALUE));

        disconnect(&mut graph, &pin(1, "fast"));

        assert!(!is_connected(&graph, &pin(1, "fast")));
        assert_eq!(graph.links, []);
    }

    #[test]
    fn moved_nodes_keep_their_wires() {
        let mut graph = graph();
        connect(&mut graph, &pin(1, "fast"), &pin(2, CONDITION));

        move_node(&mut graph, NodeId(2), vec2(5.0, -5.0));

        assert_eq!(
            graph.node(NodeId(2)).map(|node| node.position),
            Some([5.0, -5.0])
        );
        assert_eq!(graph.links.len(), 1);
    }

    #[test]
    fn renamed_pins_keep_their_wires() {
        let mut graph = graph();
        connect(&mut graph, &pin(1, "fast"), &pin(2, CONDITION));

        rename_pin(&mut graph, NodeId(1), "fast", "quick");

        assert_eq!(links(&graph), [(1, "quick", 2, CONDITION)]);
    }

    #[test]
    fn wires_of_pins_that_are_gone_are_removed() {
        let mut graph = graph();
        connect(&mut graph, &pin(1, "fast"), &pin(3, VALUE));
        connect(&mut graph, &pin(3, RESULT), &pin(2, CONDITION));
        graph.nodes[2].kind = NodeKind::Operator {
            operator: Operator::And,
        };

        remove_broken_links(&mut graph);

        assert_eq!(links(&graph), [(3, RESULT, 2, CONDITION)]);
    }
}
