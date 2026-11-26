//! StarScript's `Game`: its fields, and running the code of the nodes of
//! its tree each frame.

use std::cell::Cell;

use crate::Field;
use crate::nodes::{self, Host};

/// The fields of a game: the public ones, whether it asked to close, and
/// the root of its tree of nodes.
pub struct GameFields<H: Host> {
    /// `background`: the color behind everything.
    pub background: Field<H::Color>,
    /// `camera`: what 2D games are seen from.
    pub camera: Field<Option<H::Camera2D>>,
    /// `camera_3d`: what 3D games are seen from; it wins over `camera`.
    pub camera_3d: Field<Option<H::Camera3D>>,
    /// `ambient`: the light that reaches every 3D surface.
    pub ambient: Field<H::Color>,
    /// `ambient_2d`: the light that reaches every sprite.
    pub ambient_2d: Field<H::Color>,
    quit_requested: Cell<bool>,
    root: H::Node,
}

impl<H: Host> GameFields<H> {
    /// The fields of a new game whose tree hangs from `root`: black, seen
    /// from no camera, its sprites lit in full.
    #[must_use]
    pub fn new(root: H::Node) -> Self {
        Self {
            background: Field::new(H::color(0.0, 0.0, 0.0, 1.0)),
            camera: Field::new(None),
            camera_3d: Field::new(None),
            ambient: Field::new(H::color(0.0, 0.0, 0.0, 1.0)),
            ambient_2d: Field::new(H::color(1.0, 1.0, 1.0, 1.0)),
            quit_requested: Cell::new(false),
            root,
        }
    }

    /// The root of the game's tree.
    #[must_use]
    pub const fn root(&self) -> &H::Node {
        &self.root
    }

    /// `func quit`: closes the game after the current frame.
    pub fn quit(&self) {
        self.quit_requested.set(true);
    }

    /// Whether the game asked to close.
    #[must_use]
    pub fn is_quit_requested(&self) -> bool {
        self.quit_requested.get()
    }

    /// `func add_child`: puts `node` in the game's tree, at the top.
    pub fn add_child(&self, node: H::Node) {
        nodes::add_child::<H>(&self.root, node);
    }

    /// Makes `node` the game's `camera` or `camera_3d` if it is a camera
    /// marked `main`: called as it enters the game's tree, before its
    /// `start`.
    pub fn see_from_if_main(&self, node: &H::Node) {
        if let Some(camera) = H::as_camera_2d(node) {
            if H::camera_2d(&camera).main.get() {
                self.camera.set(Some(camera));
            }
        } else if let Some(camera) = H::as_camera_3d(node)
            && H::camera_3d(&camera).main.get()
        {
            self.camera_3d.set(Some(camera));
        }
    }
}

/// Runs a frame of the game's code: the game's `update`, then `start` (the
/// first time) and `update` of every node of its tree, parents first, each
/// node's animation moving on after its `update`.
/// Nodes added during the frame start in the next one; nodes removed
/// during it are skipped. `nodes` is a reused list, left empty.
pub fn run_frame<H: Host>(game: &H::Game, delta: f32, nodes: &mut Vec<H::Node>) {
    H::update_game(game, delta);
    let fields = H::game(game);
    let root = fields.root();
    nodes.push(root.clone());
    let mut next = 0;
    while let Some(node) = nodes.get(next).cloned() {
        nodes.extend(H::node(&node).children().iter().cloned());
        next += 1;
    }
    for node in nodes.iter() {
        if !nodes::is_same_or_ancestor::<H>(root, node) {
            continue;
        }
        if H::node(node).start_once() {
            fields.see_from_if_main(node);
            H::start(node);
        }
        H::update(node, delta);
        crate::animations::advance::<H>(node, delta);
    }
    nodes.clear();
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::*;
    use crate::node_classes::Camera2dFields;
    use crate::test_host::{TestGame, TestHost, TestNode};

    fn game() -> Rc<TestGame> {
        Rc::new(TestGame {
            fields: GameFields::new(Rc::new(TestNode::new("Game"))),
            calls: RefCell::new(Vec::new()),
        })
    }

    fn camera(name: &str, main: bool) -> Rc<TestNode> {
        let fields = Camera2dFields::new();
        fields.main.set(main);
        Rc::new(TestNode {
            camera: Some(fields),
            ..TestNode::new(name)
        })
    }

    #[test]
    fn nodes_start_once_and_update_every_frame_after_the_game() {
        let game = game();
        let player = Rc::new(TestNode::new("player"));
        game.fields.add_child(Rc::clone(&player));
        let mut nodes = Vec::new();

        run_frame::<TestHost>(&game, 0.5, &mut nodes);
        run_frame::<TestHost>(&game, 0.25, &mut nodes);

        assert_eq!(player.calls(), ["start", "update 0.5", "update 0.25"]);
        assert_eq!(*game.calls.borrow(), ["update 0.5", "update 0.25"]);
        assert!(nodes.is_empty());
    }

    #[test]
    fn nodes_removed_during_a_frame_are_skipped() {
        let game = game();
        let (first, second) = (
            Rc::new(TestNode::new("first")),
            Rc::new(TestNode::new("second")),
        );
        game.fields.add_child(Rc::clone(&first));
        game.fields.add_child(Rc::clone(&second));
        *first.removes.borrow_mut() = Some(Rc::clone(&second));

        run_frame::<TestHost>(&game, 1.0, &mut Vec::new());

        assert_eq!(first.calls(), ["start", "update 1"]);
        assert_eq!(second.calls(), Vec::<String>::new());
    }

    #[test]
    fn main_cameras_become_the_game_s_camera_as_they_start() {
        let game = game();
        let (main, other) = (camera("main", true), camera("other", false));
        game.fields.add_child(Rc::clone(&other));
        game.fields.add_child(Rc::clone(&main));

        assert!(game.fields.camera.get().is_none());
        run_frame::<TestHost>(&game, 1.0, &mut Vec::new());

        assert!(
            game.fields
                .camera
                .get()
                .is_some_and(|seen| Rc::ptr_eq(&seen, &main))
        );
    }

    #[test]
    fn games_ask_to_quit() {
        let game = game();

        assert!(!game.fields.is_quit_requested());
        game.fields.quit();

        assert!(game.fields.is_quit_requested());
    }
}
