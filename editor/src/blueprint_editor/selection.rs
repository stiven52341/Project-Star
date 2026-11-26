//! What is selected on a canvas: nodes, one picked with a click or several
//! taken with a box or with `Shift`/`Ctrl` and clicks, or else one wire,
//! picked with a click so it can be removed.

use star_blueprint::{Graph, Link, NodeId};

/// The selected nodes of a graph, in the order they were selected, or the
/// selected wire: selecting one leaves the other out.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Selection {
    nodes: Vec<NodeId>,
    wire: Option<Link>,
}

impl Selection {
    /// Only the node `id`.
    pub(super) fn only(id: NodeId) -> Self {
        Self {
            nodes: vec![id],
            wire: None,
        }
    }

    /// Only the wire `link`.
    pub(super) const fn only_wire(link: Link) -> Self {
        Self {
            nodes: Vec::new(),
            wire: Some(link),
        }
    }

    /// The selected wire, if a wire is selected.
    pub(super) const fn wire(&self) -> Option<&Link> {
        self.wire.as_ref()
    }

    /// The node selected, if it is exactly one: the inspector shows it.
    pub(super) fn single(&self) -> Option<NodeId> {
        match self.nodes.as_slice() {
            [id] => Some(*id),
            _ => None,
        }
    }

    pub(super) fn contains(&self, id: NodeId) -> bool {
        self.nodes.contains(&id)
    }

    pub(super) fn len(&self) -> usize {
        self.nodes.len()
    }

    pub(super) fn nodes(&self) -> &[NodeId] {
        &self.nodes
    }

    pub(super) fn clear(&mut self) {
        self.nodes.clear();
        self.wire = None;
    }

    /// Adds `id`, if it is not selected yet.
    pub(super) fn add(&mut self, id: NodeId) {
        self.wire = None;
        if !self.contains(id) {
            self.nodes.push(id);
        }
    }

    /// Selects `id` if it was not, and leaves it out if it was.
    pub(super) fn toggle(&mut self, id: NodeId) {
        self.wire = None;
        if self.contains(id) {
            self.nodes.retain(|selected| *selected != id);
        } else {
            self.nodes.push(id);
        }
    }

    /// Forgets the nodes and the wire that are no longer in `graph`.
    pub(super) fn keep_existing(&mut self, graph: &Graph) {
        self.nodes.retain(|id| graph.node(*id).is_some());
        if self
            .wire
            .as_ref()
            .is_some_and(|wire| !graph.links.contains(wire))
        {
            self.wire = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_one_node_is_shown_in_the_inspector() {
        let mut selection = Selection::only(NodeId(1));

        assert_eq!(selection.single(), Some(NodeId(1)));

        selection.add(NodeId(2));

        assert_eq!(selection.single(), None);
        assert_eq!(selection.len(), 2);
    }

    #[test]
    fn toggling_adds_and_leaves_out() {
        let mut selection = Selection::only(NodeId(1));

        selection.toggle(NodeId(2));
        selection.toggle(NodeId(1));

        assert_eq!(selection.nodes(), [NodeId(2)]);
    }

    fn wire() -> Link {
        Link {
            from: star_blueprint::PinRef::new(NodeId(1), "then"),
            to: star_blueprint::PinRef::new(NodeId(2), "run"),
        }
    }

    #[test]
    fn a_wire_and_nodes_are_never_selected_together() {
        let mut selection = Selection::only_wire(wire());

        assert_eq!(selection.wire(), Some(&wire()));
        assert_eq!(selection.single(), None);

        selection.add(NodeId(3));

        assert_eq!(selection.wire(), None);
        assert_eq!(selection.nodes(), [NodeId(3)]);
    }

    #[test]
    fn wires_no_longer_in_the_graph_are_forgotten() {
        let mut graph = Graph {
            name: "update".to_owned(),
            parameters: Vec::new(),
            returns: None,
            is_override: false,
            private: false,
            nodes: Vec::new(),
            links: vec![wire()],
        };
        let mut selection = Selection::only_wire(wire());

        selection.keep_existing(&graph);
        assert_eq!(selection.wire(), Some(&wire()));

        graph.links.clear();
        selection.keep_existing(&graph);
        assert_eq!(selection.wire(), None);
    }

    #[test]
    fn nodes_are_added_once() {
        let mut selection = Selection::default();

        selection.add(NodeId(3));
        selection.add(NodeId(3));

        assert_eq!(selection.nodes(), [NodeId(3)]);
    }
}
