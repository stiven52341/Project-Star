//! The nodes an animator animates, named as `.anim` tracks name them: by
//! their path from the animator's parent (decision 79).

use star_engine::star_scene::{NodeOutline, PATH_SEPARATOR};

/// The node an animator at `animator` animates: its parent.
pub(super) fn animated_root(nodes: &[NodeOutline], animator: usize) -> Option<usize> {
    nodes.get(animator)?.parent
}

/// The path of the node at `node` from the node at `root`: `""` for the
/// root itself; `None` if it is not under it.
pub(super) fn path_of(nodes: &[NodeOutline], root: usize, node: usize) -> Option<String> {
    let mut names = Vec::new();
    let mut current = node;
    while current != root {
        let outline = nodes.get(current)?;
        names.push(outline.name.as_str());
        current = outline.parent?;
    }
    names.reverse();
    Some(names.join(&PATH_SEPARATOR.to_string()))
}

/// The node at `path` from the node at `root`: through the first child of
/// each name, as the game finds it.
pub(super) fn node_at(nodes: &[NodeOutline], root: usize, path: &str) -> Option<usize> {
    if path.is_empty() {
        return (root < nodes.len()).then_some(root);
    }
    path.split(PATH_SEPARATOR).try_fold(root, |parent, name| {
        nodes
            .iter()
            .position(|node| node.parent == Some(parent) && node.name == name)
    })
}

/// The nodes an animator can animate: `root` and every node under it, with
/// their paths, in the scene's order.
pub(super) fn animated_nodes(nodes: &[NodeOutline], root: usize) -> Vec<(usize, String)> {
    (root..nodes.len())
        .filter_map(|index| path_of(nodes, root, index).map(|path| (index, path)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::ProjectDimension;
    use crate::scene_editor::components::new_node;

    /// World > Knight > (Arm > Hand, Animator).
    fn knight() -> Vec<NodeOutline> {
        let node = |name: &str, parent: Option<usize>| NodeOutline {
            parent,
            ..new_node(name.to_owned(), ProjectDimension::TwoD)
        };
        vec![
            node("World", None),
            node("Knight", Some(0)),
            node("Arm", Some(1)),
            node("Hand", Some(2)),
            node("Animator", Some(1)),
        ]
    }

    #[test]
    fn paths_start_at_the_animators_parent() {
        let nodes = knight();
        let root = animated_root(&nodes, 4).unwrap();

        assert_eq!(root, 1);
        assert_eq!(path_of(&nodes, root, 1).as_deref(), Some(""));
        assert_eq!(path_of(&nodes, root, 3).as_deref(), Some("Arm/Hand"));
        assert_eq!(path_of(&nodes, root, 0), None);
        assert_eq!(node_at(&nodes, root, "Arm/Hand"), Some(3));
        assert_eq!(node_at(&nodes, root, ""), Some(1));
        assert_eq!(node_at(&nodes, root, "Leg"), None);
    }

    #[test]
    fn animators_animate_their_parent_and_what_is_under_it() {
        let nodes = knight();

        let paths: Vec<String> = animated_nodes(&nodes, 1)
            .into_iter()
            .map(|(_, path)| path)
            .collect();

        assert_eq!(paths, ["", "Arm", "Arm/Hand", "Animator"]);
    }
}
