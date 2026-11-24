//! The tree of a scene's nodes, built from the parent each node names.

use star_engine::star_scene::NodeOutline;

/// The children of every node, in file order: `children[i]` lists the
/// positions of the nodes whose parent is node `i`.
pub(super) fn children_of(nodes: &[NodeOutline]) -> Vec<Vec<usize>> {
    let mut children = vec![Vec::new(); nodes.len()];
    for (index, node) in nodes.iter().enumerate() {
        if let Some(siblings) = node.parent.and_then(|parent| children.get_mut(parent)) {
            siblings.push(index);
        }
    }
    children
}

/// Whether the node at `node` can be moved under the node at `parent`:
/// not the root, not under the parent it already has, and not into
/// itself or one of its descendants.
pub(super) fn can_move(nodes: &[NodeOutline], node: usize, parent: usize) -> bool {
    let Some(moved) = nodes.get(node) else {
        return false;
    };
    if moved.parent.is_none() || moved.parent == Some(parent) || parent >= nodes.len() {
        return false;
    }
    let mut ancestor = Some(parent);
    while let Some(index) = ancestor {
        if index == node {
            return false;
        }
        ancestor = nodes.get(index).and_then(|node| node.parent);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use star_engine::prelude::SceneOutline;

    /// Root, two children of the root, and a grandchild under the second.
    fn nodes() -> Vec<NodeOutline> {
        SceneOutline::from_text(
            "format = 1\n\
             [[node]]\nname = \"Root\"\n\
             [[node]]\nname = \"A\"\nparent = 0\n\
             [[node]]\nname = \"B\"\nparent = 0\n\
             [[node]]\nname = \"C\"\nparent = 2\n",
        )
        .unwrap()
        .into_nodes()
    }

    #[test]
    fn nodes_move_anywhere_but_into_themselves() {
        let nodes = nodes();

        assert!(can_move(&nodes, 3, 1));
        assert!(can_move(&nodes, 2, 1));
        assert!(!can_move(&nodes, 3, 2), "already its parent");
        assert!(!can_move(&nodes, 2, 3), "inside itself");
        assert!(!can_move(&nodes, 2, 2), "itself");
        assert!(!can_move(&nodes, 0, 1), "the root");
        assert!(!can_move(&nodes, 1, 9), "no such parent");
    }

    #[test]
    fn children_are_listed_under_their_parent_in_file_order() {
        assert_eq!(
            children_of(&nodes()),
            vec![vec![1, 2], vec![], vec![3], vec![]]
        );
    }
}
