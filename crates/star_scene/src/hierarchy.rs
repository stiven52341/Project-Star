//! The node tree: names, parents and children of the entities of a world.

use star_core::{EngineError, EngineResult};
use star_ecs::{Entity, World};

/// The name of a node, shown in the editor and used to find it among its
/// siblings.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Name(pub String);

/// The tags of a node: words that groups of nodes share, such as `enemy`,
/// to find them all at once.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tags(pub Vec<String>);

impl Tags {
    /// Whether one of the tags is `tag`.
    #[must_use]
    pub fn contains(&self, tag: &str) -> bool {
        self.0.iter().any(|own| own == tag)
    }
}

impl Name {
    /// The name as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The node a node belongs to. Changed only through [`Hierarchy`], which
/// keeps it in step with the parent's [`Children`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Parent(Entity);

impl Parent {
    /// The parent node.
    #[must_use]
    pub const fn entity(self) -> Entity {
        self.0
    }
}

/// The nodes that belong to a node, in order. Changed only through
/// [`Hierarchy`], which keeps it in step with the children's [`Parent`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Children(Vec<Entity>);

impl Children {
    /// The children, in the order they were added.
    #[must_use]
    pub fn as_slice(&self) -> &[Entity] {
        &self.0
    }
}

/// The node tree of a [`World`]: a node is an entity with a [`Name`], which
/// may have a parent and children. Any entity can join the tree, and every
/// other component still works on nodes.
///
/// Destroy nodes with [`Hierarchy::despawn_node`]: [`World::despawn`] knows
/// nothing about the tree, so it would leave the node in its parent's children
/// and its own children without their parent.
pub trait Hierarchy {
    /// Creates a node called `name`, as the last child of `parent` (or as a
    /// root, without one).
    ///
    /// # Errors
    ///
    /// Returns an error if `parent` is not alive, or there is no room for
    /// more entities.
    fn spawn_node(&mut self, name: &str, parent: Option<Entity>) -> EngineResult<Entity>;

    /// Moves `node`, with its whole subtree, to be the last child of `parent`
    /// (or a root, without one).
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Scene`] if a node is not alive, or `parent` is
    /// `node` or one of its descendants (the tree would become a loop).
    fn set_parent(&mut self, node: Entity, parent: Option<Entity>) -> EngineResult<()>;

    /// The parent of `node`; `None` for roots.
    fn parent(&self, node: Entity) -> Option<Entity>;

    /// A copy of the children of `node`, in order.
    fn children(&self, node: Entity) -> Vec<Entity>;

    /// The first child of `node` called `name`.
    fn find_child(&self, node: Entity, name: &str) -> Option<Entity>;

    /// Destroys `node` and its whole subtree, and takes it out of its
    /// parent's children. Returns how many nodes were destroyed (0 if `node`
    /// was not alive).
    fn despawn_node(&mut self, node: Entity) -> usize;
}

impl Hierarchy for World {
    fn spawn_node(&mut self, name: &str, parent: Option<Entity>) -> EngineResult<Entity> {
        if let Some(parent) = parent {
            ensure_alive(self, parent)?;
        }
        let node = self.spawn()?;
        self.insert(node, Name(name.to_owned()))?;
        if let Some(parent) = parent {
            attach(self, node, parent)?;
        }
        Ok(node)
    }

    fn set_parent(&mut self, node: Entity, parent: Option<Entity>) -> EngineResult<()> {
        ensure_alive(self, node)?;
        if let Some(parent) = parent {
            ensure_alive(self, parent)?;
            if is_same_or_ancestor(self, node, parent) {
                return Err(EngineError::Scene(format!(
                    "{} cannot be moved under {}, which is itself or one of its descendants",
                    describe(self, node),
                    describe(self, parent)
                )));
            }
        }
        detach(self, node);
        match parent {
            Some(parent) => attach(self, node, parent),
            None => Ok(()),
        }
    }

    fn parent(&self, node: Entity) -> Option<Entity> {
        self.get::<Parent>(node).map(|parent| parent.entity())
    }

    fn children(&self, node: Entity) -> Vec<Entity> {
        self.get::<Children>(node)
            .map(|children| children.0.clone())
            .unwrap_or_default()
    }

    fn find_child(&self, node: Entity, name: &str) -> Option<Entity> {
        self.children(node).into_iter().find(|&child| {
            self.get::<Name>(child)
                .is_some_and(|child_name| child_name.as_str() == name)
        })
    }

    fn despawn_node(&mut self, node: Entity) -> usize {
        if !self.is_alive(node) {
            return 0;
        }
        detach(self, node);
        let mut pending = vec![node];
        let mut despawned = 0;
        while let Some(next) = pending.pop() {
            pending.extend(self.children(next));
            if self.despawn(next) {
                despawned += 1;
            }
        }
        despawned
    }
}

fn ensure_alive(world: &World, node: Entity) -> EngineResult<()> {
    if world.is_alive(node) {
        Ok(())
    } else {
        Err(EngineError::Scene(format!("{node:?} is not alive")))
    }
}

/// Whether `node` is `other` or one of its ancestors.
fn is_same_or_ancestor(world: &World, node: Entity, other: Entity) -> bool {
    let mut current = Some(other);
    while let Some(ancestor) = current {
        if ancestor == node {
            return true;
        }
        current = world.parent(ancestor);
    }
    false
}

/// Makes `node`, which has no parent, the last child of `parent`.
fn attach(world: &mut World, node: Entity, parent: Entity) -> EngineResult<()> {
    world.insert(node, Parent(parent))?;
    let added = world
        .get_mut::<Children>(parent)
        .map(|mut children| children.0.push(node))
        .is_some();
    if !added {
        world.insert(parent, Children(vec![node]))?;
    }
    Ok(())
}

/// Takes `node` out of its parent's children, making it a root.
fn detach(world: &mut World, node: Entity) {
    let Some(Parent(parent)) = world.remove::<Parent>(node) else {
        return;
    };
    let now_empty = world
        .get_mut::<Children>(parent)
        .is_some_and(|mut children| {
            children.0.retain(|&child| child != node);
            children.0.is_empty()
        });
    if now_empty {
        world.remove::<Children>(parent);
    }
}

/// The node's name for error messages, or its handle if it has none.
fn describe(world: &World, node: Entity) -> String {
    world.get::<Name>(node).map_or_else(
        || format!("{node:?}"),
        |name| format!("\"{}\"", name.as_str()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A world with `Level`, its children `Hero` and `Enemy`, and `Sword`
    /// under `Hero`.
    struct Tree {
        world: World,
        level: Entity,
        hero: Entity,
        enemy: Entity,
        sword: Entity,
    }

    fn tree() -> Tree {
        let mut world = World::new();
        let level = world.spawn_node("Level", None).unwrap();
        let hero = world.spawn_node("Hero", Some(level)).unwrap();
        let enemy = world.spawn_node("Enemy", Some(level)).unwrap();
        let sword = world.spawn_node("Sword", Some(hero)).unwrap();
        Tree {
            world,
            level,
            hero,
            enemy,
            sword,
        }
    }

    #[test]
    fn nodes_know_their_name_parent_and_children_in_order() {
        let Tree {
            world,
            level,
            hero,
            enemy,
            ..
        } = tree();

        assert_eq!(world.get::<Name>(hero).unwrap().as_str(), "Hero");
        assert_eq!(world.parent(hero), Some(level));
        assert_eq!(world.parent(level), None);
        assert_eq!(world.children(level), [hero, enemy]);
        assert_eq!(world.children(enemy), []);
    }

    #[test]
    fn children_are_found_by_name() {
        let Tree {
            world,
            level,
            enemy,
            ..
        } = tree();

        assert_eq!(world.find_child(level, "Enemy"), Some(enemy));
        assert_eq!(world.find_child(level, "Sword"), None);
    }

    #[test]
    fn moving_a_node_updates_both_parents() {
        let Tree {
            mut world,
            level,
            hero,
            enemy,
            sword,
        } = tree();

        world.set_parent(sword, Some(enemy)).unwrap();

        assert_eq!(world.parent(sword), Some(enemy));
        assert_eq!(world.children(enemy), [sword]);
        assert_eq!(world.children(hero), []);
        assert!(!world.has::<Children>(hero));
        assert_eq!(world.children(level), [hero, enemy]);
    }

    #[test]
    fn nodes_can_become_roots() {
        let Tree {
            mut world,
            level,
            hero,
            enemy,
            ..
        } = tree();

        world.set_parent(hero, None).unwrap();

        assert_eq!(world.parent(hero), None);
        assert_eq!(world.children(level), [enemy]);
    }

    #[test]
    fn loops_are_rejected() {
        let Tree {
            mut world,
            level,
            hero,
            sword,
            ..
        } = tree();

        for (node, parent) in [(hero, hero), (level, sword), (hero, sword)] {
            let error = world.set_parent(node, Some(parent)).unwrap_err();

            assert!(matches!(error, EngineError::Scene(_)), "{error}");
        }
        assert_eq!(world.parent(sword), Some(hero));
    }

    #[test]
    fn dead_parents_are_rejected() {
        let mut world = World::new();
        let gone = world.spawn_node("Gone", None).unwrap();
        world.despawn_node(gone);

        assert!(world.spawn_node("Orphan", Some(gone)).is_err());
        assert_eq!(world.entity_count(), 0);
    }

    #[test]
    fn despawning_a_node_destroys_its_subtree() {
        let Tree {
            mut world,
            level,
            hero,
            enemy,
            sword,
        } = tree();

        assert_eq!(world.despawn_node(hero), 2);

        assert!(!world.is_alive(hero));
        assert!(!world.is_alive(sword));
        assert_eq!(world.children(level), [enemy]);
        assert_eq!(world.despawn_node(hero), 0);
    }

    #[test]
    fn despawning_a_root_destroys_the_whole_tree() {
        let Tree {
            mut world, level, ..
        } = tree();

        assert_eq!(world.despawn_node(level), 4);
        assert_eq!(world.entity_count(), 0);
    }
}
