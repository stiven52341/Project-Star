use std::f32::consts::FRAC_PI_2;
use std::rc::Rc;

use super::*;
use crate::test_host::{TestHost, TestNode};

type Node = Rc<TestNode>;

fn node(name: &str) -> Node {
    Rc::new(TestNode::new(name))
}

fn node_3d(name: &str) -> Node {
    Rc::new(TestNode::new_3d(name))
}

fn names(nodes: &[Node]) -> Vec<String> {
    nodes.iter().map(|node| node.node.name.get()).collect()
}

fn near(value: f32, expected: f32) -> bool {
    (value - expected).abs() < 1e-5
}

/// root ─ a ─ c
///      └ b
fn tree() -> (Node, Node, Node, Node) {
    let (root, a, b, c) = (node("root"), node("a"), node("b"), node("c"));
    add_child::<TestHost>(&root, Rc::clone(&a));
    add_child::<TestHost>(&root, Rc::clone(&b));
    add_child::<TestHost>(&a, Rc::clone(&c));
    (root, a, b, c)
}

#[test]
fn every_node_has_its_own_number() {
    assert_ne!(node("a").node.id(), node("a").node.id());
}

#[test]
fn children_are_added_in_order_and_know_their_parent() {
    let (root, a, _, c) = tree();

    assert_eq!(names(&children::<TestHost>(&root).to_vec()), ["a", "b"]);
    assert!(parent::<TestHost>(&c).is_some_and(|parent| Rc::ptr_eq(&parent, &a)));
    assert!(parent::<TestHost>(&root).is_none());
}

#[test]
fn adding_a_child_moves_it_from_its_old_parent() {
    let (_, a, b, c) = tree();

    add_child::<TestHost>(&b, Rc::clone(&c));

    assert!(children::<TestHost>(&a).is_empty());
    assert_eq!(names(&children::<TestHost>(&b).to_vec()), ["c"]);
}

#[test]
fn a_node_cannot_go_under_its_own_descendant() {
    let (root, a, _, c) = tree();

    add_child::<TestHost>(&c, Rc::clone(&a));
    add_child::<TestHost>(&a, Rc::clone(&a));

    assert!(parent::<TestHost>(&a).is_some_and(|parent| Rc::ptr_eq(&parent, &root)));
    assert!(children::<TestHost>(&c).is_empty());
}

#[test]
fn removed_nodes_leave_the_tree_with_their_children() {
    let (root, a, _, c) = tree();

    remove::<TestHost>(&a);

    assert_eq!(names(&children::<TestHost>(&root).to_vec()), ["b"]);
    assert!(parent::<TestHost>(&a).is_none());
    assert!(is_same_or_ancestor::<TestHost>(&a, &c));
}

#[test]
fn nodes_are_found_by_name_and_tag_parents_first() {
    let (root, a, b, c) = tree();
    b.node.tags.set(List::from_vec(vec!["enemy".to_owned()]));
    c.node.tags.set(List::from_vec(vec!["enemy".to_owned()]));

    assert_eq!(names(&descendants::<TestHost>(&root)), ["a", "c", "b"]);
    assert!(find_child::<TestHost>(&root, "c").is_none());
    assert!(find_by_name::<TestHost>(&root, "c").is_some_and(|found| Rc::ptr_eq(&found, &c)));
    assert_eq!(
        names(&find_by_tag::<TestHost>(&root, "enemy").to_vec()),
        ["c", "b"]
    );
    assert!(has_tag::<TestHost>(&b, "enemy"));
    assert!(!has_tag::<TestHost>(&a, "enemy"));
}

#[test]
fn global_places_add_the_ancestors_places() {
    let (_, a, _, c) = tree();
    a.node.x.set(10.0);
    a.node.rotation.set(FRAC_PI_2);
    a.node.scale_x.set(2.0);
    c.node.x.set(1.0);

    let [across, up, depth] = global_position::<TestHost>(&c);

    assert!(near(across, 10.0) && near(up, 2.0) && near(depth, 0.0));
    assert!(near(global_rotation::<TestHost>(&c), FRAC_PI_2));
}

#[test]
fn global_places_can_be_set() {
    let (_, a, _, c) = tree();
    a.node.x.set(10.0);
    a.node.rotation.set(FRAC_PI_2);

    set_global_position::<TestHost>(&c, 10.0, 3.0);
    set_global_rotation::<TestHost>(&c, 0.0);
    translate_global::<TestHost>(&c, 0.0, 1.0);

    assert!(near(c.node.x.get(), 4.0) && near(c.node.y.get(), 0.0));
    assert!(near(c.node.rotation.get(), -FRAC_PI_2));
}

#[test]
fn local_moves_and_turns_add_up() {
    let a = node("a");

    translate::<TestHost>(&a, 1.0, 2.0);
    translate::<TestHost>(&a, 1.0, 2.0);
    rotate::<TestHost>(&a, 0.5);

    assert!(near(a.node.x.get(), 2.0) && near(a.node.y.get(), 4.0));
    assert!(near(a.node.rotation.get(), 0.5));
}

#[test]
fn nodes_3d_are_placed_in_3d_ancestors_only() {
    let (flat, outer, inner) = (node("flat"), node_3d("outer"), node_3d("inner"));
    flat.node.x.set(100.0);
    add_child::<TestHost>(&flat, Rc::clone(&outer));
    add_child::<TestHost>(&outer, Rc::clone(&inner));
    outer.node_3d.z.set(5.0);
    inner.node.x.set(1.0);

    assert_eq!(global_position::<TestHost>(&inner), [1.0, 0.0, 5.0]);

    set_global_position_3d::<TestHost>(&inner, [0.0, 0.0, 0.0]);

    assert!(near(inner.node.x.get(), 0.0));
    assert!(near(inner.node_3d.z.get(), -5.0));
}

#[test]
fn nodes_start_once() {
    let a = node("a");

    assert!(a.node.start_once());
    assert!(!a.node.start_once());
}

#[test]
fn nodes_3d_move_and_look_in_3d() {
    let (eye, flat) = (node_3d("eye"), node("flat"));

    translate_3d::<TestHost>(&eye, 1.0, 0.0, 2.0);
    translate_3d::<TestHost>(&flat, 1.0, 0.0, 2.0);
    translate_global_3d::<TestHost>(&eye, 0.0, 1.0, 0.0);
    look_at::<TestHost>(&eye, 1.0, 1.0, 0.0);

    assert_eq!(global_position::<TestHost>(&eye), [1.0, 1.0, 2.0]);
    assert!(near(flat.node.x.get(), 0.0), "only nodes 3D move in 3D");
    assert!(near(eye.node_3d.pitch.get(), 0.0) && near(eye.node_3d.yaw.get(), 0.0));

    look_at::<TestHost>(&eye, 2.0, 1.0, 2.0);

    assert!(near(eye.node_3d.yaw.get(), -FRAC_PI_2));
}
