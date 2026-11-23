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
    fn children_are_listed_under_their_parent_in_file_order() {
        assert_eq!(
            children_of(&nodes()),
            vec![vec![1, 2], vec![], vec![3], vec![]]
        );
    }
}
