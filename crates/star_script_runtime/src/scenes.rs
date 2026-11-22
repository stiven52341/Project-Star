//! The scenes StarScript's `Scene.load` reads.
//!
//! Reading a `.scene` file needs the engine, which this crate cannot depend
//! on, and the generated `Scene` class only uses this crate, so console
//! programs keep working without the engine. So a game registers a loader
//! when it starts ([`set_loader`]), which reads files through the engine and
//! describes their nodes with the plain types of this module; [`load`] calls
//! it and keeps each result, so a scene used many times (a bullet, an enemy)
//! is read once. The game runs on one thread, so all of it is per thread.

use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Write as _;
use std::rc::Rc;

/// One node of a scene, as the objects of the game are built from it. What
/// the file does not write is `None`, so it keeps the value the object's
/// `init` gave it.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneNode {
    /// Its name.
    pub name: String,
    /// The StarScript class it is built as; `None` for a plain `Node`, or a
    /// `Sprite` if it shows an image.
    pub class: Option<String>,
    /// Position of its parent in the scene's list; only the root, the first
    /// node, has none.
    pub parent: Option<usize>,
    /// Where it is, relative to its parent.
    pub transform: Option<SceneTransform>,
    /// The image it shows; only `Sprite` objects can show one.
    pub sprite: Option<SceneSprite>,
}

/// Where a [`SceneNode`] is, relative to its parent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneTransform {
    /// Horizontal position.
    pub x: f32,
    /// Vertical position.
    pub y: f32,
    /// Rotation in radians, counterclockwise.
    pub rotation: f32,
    /// Horizontal scale.
    pub scale_x: f32,
    /// Vertical scale.
    pub scale_y: f32,
}

/// The image a [`SceneNode`] shows.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneSprite {
    /// Asset path of the image.
    pub texture: String,
    /// Width in world units.
    pub width: f32,
    /// Height in world units.
    pub height: f32,
    /// Sprites in higher layers are drawn on top of lower ones.
    pub layer: i64,
    /// Whether the pixels are kept sharp.
    pub pixelated: bool,
    /// Red, green, blue and opacity, from `0.0` to `1.0`.
    pub color: [f32; 4],
}

/// Reads the scene at an asset path, or explains why it cannot.
type Loader = Box<dyn Fn(&str) -> Result<Vec<SceneNode>, String>>;

/// A loaded scene, or `None` if it could not be loaded.
type Loaded = Option<Rc<[SceneNode]>>;

thread_local! {
    static LOADER: RefCell<Option<Loader>> = const { RefCell::new(None) };
    static LOADED: RefCell<HashMap<String, Loaded>> = RefCell::new(HashMap::new());
}

/// Sets how scenes are read. Called once, before the game is created.
pub fn set_loader(loader: impl Fn(&str) -> Result<Vec<SceneNode>, String> + 'static) {
    LOADER.with(|current| {
        if let Ok(mut current) = current.try_borrow_mut() {
            *current = Some(Box::new(loader));
        }
        LOADED.with(|loaded| {
            if let Ok(mut loaded) = loaded.try_borrow_mut() {
                loaded.clear();
            }
        });
    });
}

/// The nodes of the scene at `path`, parents first. `None` if it cannot be
/// loaded: the reason is written once to the error output, and the game
/// goes on. Each path is read only the first time.
#[must_use]
pub fn load(path: &str) -> Option<Rc<[SceneNode]>> {
    if let Some(loaded) = LOADED.with(|loaded| loaded.borrow().get(path).cloned()) {
        return loaded;
    }
    let result = LOADER.with(|loader| match &*loader.borrow() {
        Some(loader) => loader(path),
        None => Err("scenes can only be loaded while a game runs".to_owned()),
    });
    let loaded = match result {
        Ok(nodes) if !nodes.is_empty() => Some(Rc::from(nodes)),
        Ok(_) => {
            warn(path, "it has no nodes");
            None
        }
        Err(reason) => {
            warn(path, &reason);
            None
        }
    };
    LOADED.with(|cache| {
        cache.borrow_mut().insert(path.to_owned(), loaded.clone());
    });
    loaded
}

/// Marks the scene at `path` as one that cannot be loaded, because its
/// nodes could not be built (e.g. a class that does not exist): the reason
/// is written once, and [`load`] gives `None` for it from then on.
pub fn reject(path: &str, reason: &str) {
    warn(path, reason);
    LOADED.with(|cache| {
        cache.borrow_mut().insert(path.to_owned(), None);
    });
}

fn warn(path: &str, reason: &str) {
    // Nothing can be reported if the error output itself fails.
    let _ = writeln!(
        std::io::stderr(),
        "warning: cannot load the scene \"{path}\": {reason}"
    );
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    fn node(name: &str, parent: Option<usize>) -> SceneNode {
        SceneNode {
            name: name.to_owned(),
            class: None,
            parent,
            transform: None,
            sprite: None,
        }
    }

    #[test]
    fn without_a_game_no_scene_loads() {
        // Each test runs on its own thread, so no loader is set here.
        assert!(load("levels/one.scene").is_none());
    }

    #[test]
    fn scenes_are_read_once() {
        let reads = Rc::new(Cell::new(0));
        let counter = Rc::clone(&reads);
        set_loader(move |path| {
            counter.set(counter.get() + 1);
            Ok(vec![node(path, None), node("Child", Some(0))])
        });

        let first = load("levels/one.scene").unwrap();
        let second = load("levels/one.scene").unwrap();

        assert_eq!(first[0].name, "levels/one.scene");
        assert_eq!(first.len(), 2);
        assert!(Rc::ptr_eq(&first, &second));
        assert_eq!(reads.get(), 1);
    }

    #[test]
    fn failures_are_reported_once() {
        let reads = Rc::new(Cell::new(0));
        let counter = Rc::clone(&reads);
        set_loader(move |_| {
            counter.set(counter.get() + 1);
            Err("missing".to_owned())
        });

        assert!(load("nothing.scene").is_none());
        assert!(load("nothing.scene").is_none());
        assert_eq!(reads.get(), 1);
    }

    #[test]
    fn rejected_scenes_are_not_loaded_again() {
        set_loader(|path| Ok(vec![node(path, None)]));
        assert!(load("broken.scene").is_some());

        reject("broken.scene", "there is no class \"Rokc\"");

        assert!(load("broken.scene").is_none());
    }

    #[test]
    fn empty_scenes_are_not_loaded() {
        set_loader(|_| Ok(Vec::new()));

        assert!(load("empty.scene").is_none());
    }
}
