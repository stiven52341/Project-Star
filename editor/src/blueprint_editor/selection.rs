//! The nodes selected on a canvas: one picked with a click, or several
//! taken with a box or with `Shift`/`Ctrl` and clicks.

use star_blueprint::{Graph, NodeId};

/// The selected nodes of a graph, in the order they were selected.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Selection {
    nodes: Vec<NodeId>,
}

impl Selection {
    /// Only the node `id`.
    pub(super) fn only(id: NodeId) -> Self {
        Self { nodes: vec![id] }
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
    }

    /// Adds `id`, if it is not selected yet.
    pub(super) fn add(&mut self, id: NodeId) {
        if !self.contains(id) {
            self.nodes.push(id);
        }
    }

    /// Selects `id` if it was not, and leaves it out if it was.
    pub(super) fn toggle(&mut self, id: NodeId) {
        if self.contains(id) {
            self.nodes.retain(|selected| *selected != id);
        } else {
            self.nodes.push(id);
        }
    }

    /// Forgets the nodes that are no longer in `graph`.
    pub(super) fn keep_existing(&mut self, graph: &Graph) {
        self.nodes.retain(|id| graph.node(*id).is_some());
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

    #[test]
    fn nodes_are_added_once() {
        let mut selection = Selection::default();

        selection.add(NodeId(3));
        selection.add(NodeId(3));

        assert_eq!(selection.nodes(), [NodeId(3)]);
    }
}
