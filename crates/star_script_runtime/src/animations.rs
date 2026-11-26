//! Sprites that play the clips of `.frames` files (decision 78).
//!
//! A `Sprite` names a `.frames` file in its `frames` field. Every frame,
//! after the sprite's `update`, [`advance`] gives it the file's image and
//! cells, and the cell its clip is on. The files are read through the
//! engine, which sets the loader when the game starts ([`set_loader`]);
//! each file is read once. The game runs on one thread, so all of it is per
//! thread.

use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Write as _;
use std::rc::Rc;

use crate::node_classes::SpriteFields;
use crate::nodes::Host;

/// The animations of a sprite sheet, with the words and values of
/// StarScript's `Sprite`.
#[derive(Debug, Clone, PartialEq)]
pub struct SheetClips {
    /// Asset path of the image.
    pub texture: String,
    /// How many cells wide the image is.
    pub columns: i32,
    /// How many cells tall the image is.
    pub rows: i32,
    /// The animations, each with a different name.
    pub clips: Vec<SheetClip>,
}

/// One animation of a [`SheetClips`].
#[derive(Debug, Clone, PartialEq)]
pub struct SheetClip {
    /// What scripts play it by.
    pub name: String,
    /// The cells shown, in order; never empty.
    pub frames: Vec<i32>,
    /// Frames shown per second; above zero.
    pub speed: f32,
    /// Whether it starts over after its last frame, or stays on it.
    pub looping: bool,
}

impl SheetClips {
    fn clip(&self, name: &str) -> Option<&SheetClip> {
        self.clips.iter().find(|clip| clip.name == name)
    }
}

/// Reads the `.frames` file at an asset path, or explains why it cannot.
type Loader = Box<dyn Fn(&str) -> Result<SheetClips, String>>;

/// A read file, or `None` if it could not be read.
type Loaded = Option<Rc<SheetClips>>;

thread_local! {
    static LOADER: RefCell<Option<Loader>> = const { RefCell::new(None) };
    static LOADED: RefCell<HashMap<String, Loaded>> = RefCell::new(HashMap::new());
}

/// Sets how `.frames` files are read. Called once, before the game is
/// created.
pub fn set_loader(loader: impl Fn(&str) -> Result<SheetClips, String> + 'static) {
    LOADER.with(|current| {
        if let Ok(mut current) = current.try_borrow_mut() {
            *current = Some(Box::new(loader));
        }
    });
    LOADED.with(|loaded| {
        if let Ok(mut loaded) = loaded.try_borrow_mut() {
            loaded.clear();
        }
    });
}

/// `func play`: plays the clip `name` of the sprite's `frames` from its
/// first frame, unless it is already playing it.
pub fn play<H: Host>(node: &H::Node, name: &str) {
    H::with_sprite(node, |sprite| {
        let same = sprite.playing.get() && sprite.animation.read(|current| current == name);
        if !same {
            sprite.animation.set(name.to_owned());
            sprite.clip_time.set(0.0);
            sprite.playing.set(true);
        }
    });
}

/// `func stop`: stops the sprite's clip on the frame it shows.
pub fn stop<H: Host>(node: &H::Node) {
    H::with_sprite(node, |sprite| sprite.playing.set(false));
}

/// `func is_playing`: whether the sprite's clip is playing; a clip that
/// does not loop stops on its last frame.
#[must_use]
pub fn is_playing<H: Host>(node: &H::Node) -> bool {
    H::with_sprite(node, |sprite| sprite.playing.get()).unwrap_or(false)
}

/// `func animation`: the clip the sprite plays, or played last; empty
/// before any.
#[must_use]
pub fn animation<H: Host>(node: &H::Node) -> String {
    H::with_sprite(node, |sprite| sprite.animation.get()).unwrap_or_default()
}

/// Moves the clip of `node`, if it is a sprite with `frames`, `delta`
/// seconds on: it shows the file's image and cells, and the cell its clip
/// is on. Called after the node's `update`, so a clip played there shows
/// its first frame in the same frame. When a clip that does not loop ends,
/// the node's `animation_finished` runs with its name.
pub fn advance<H: Host>(node: &H::Node, delta: f32) {
    let finished = H::with_sprite(node, |sprite| advance_sprite(sprite, delta)).flatten();
    if let Some(name) = finished {
        H::animation_finished(node, &name);
    }
}

/// [`advance`] on the fields of a sprite; the name of the clip that ended,
/// if one did.
fn advance_sprite<H: Host>(sprite: &SpriteFields<H>, delta: f32) -> Option<String> {
    let sheet = sprite
        .frames
        .read(|path| if path.is_empty() { None } else { load(path) })?;
    show_sheet(sprite, &sheet);
    if !sprite.playing.get() {
        return None;
    }
    let found = sprite.animation.read(|name| {
        let clip = sheet.clip(name);
        if clip.is_none() {
            sprite.frames.read(|path| warn_missing_clip(path, name));
        }
        clip
    });
    let Some(clip) = found else {
        sprite.playing.set(false);
        return None;
    };
    let (index, time) = clip_position(clip, sprite.clip_time.get());
    let last = clip.frames.len().saturating_sub(1);
    if let Some(&frame) = clip.frames.get(index.min(last)) {
        sprite.frame.set(frame);
    }
    sprite.clip_time.set(time + delta);
    let ended = !clip.looping && index >= clip.frames.len();
    ended.then(|| {
        sprite.playing.set(false);
        clip.name.clone()
    })
}

/// The frame of `clip` shown after `time` seconds (past its end for a
/// clip that does not loop and has ended), and the time kept for it: a
/// looping clip's time goes round its length, so it never grows without
/// end.
fn clip_position(clip: &SheetClip, time: f32) -> (usize, f32) {
    #[expect(
        clippy::cast_precision_loss,
        reason = "a clip has far fewer frames than f32 counts exactly"
    )]
    let length = clip.frames.len() as f32 / clip.speed;
    let time = if clip.looping {
        time.rem_euclid(length)
    } else {
        time
    };
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the time and the speed are never negative, and the frame is rounded down on purpose"
    )]
    let shown = (time * clip.speed).floor() as usize;
    let index = if clip.looping {
        shown % clip.frames.len()
    } else {
        shown
    };
    (index, time)
}

/// Gives `sprite` the image and cells of `sheet`, changing only what
/// differs, so nothing is copied while it stays the same.
fn show_sheet<H: Host>(sprite: &SpriteFields<H>, sheet: &SheetClips) {
    if sprite.texture.read(|texture| *texture != sheet.texture) {
        sprite.texture.set(sheet.texture.clone());
    }
    sprite.columns.set(sheet.columns);
    sprite.rows.set(sheet.rows);
}

/// The `.frames` file at `path`, read the first time it is needed. `None`
/// if it cannot be read: the reason is written once.
fn load(path: &str) -> Loaded {
    if let Some(loaded) = LOADED.with(|loaded| loaded.borrow().get(path).cloned()) {
        return loaded;
    }
    let result = LOADER.with(|loader| match &*loader.borrow() {
        Some(loader) => loader(path),
        None => Err("animations can only be loaded while a game runs".to_owned()),
    });
    let loaded = match result {
        Ok(sheet) => Some(Rc::new(sheet)),
        Err(reason) => {
            warn(&format!("cannot load the frames \"{path}\": {reason}"));
            None
        }
    };
    LOADED.with(|cache| {
        cache.borrow_mut().insert(path.to_owned(), loaded.clone());
    });
    loaded
}

fn warn_missing_clip(path: &str, name: &str) {
    warn(&format!("the frames \"{path}\" have no clip \"{name}\""));
}

fn warn(message: &str) {
    // Nothing can be reported if the error output itself fails.
    let _ = writeln!(std::io::stderr(), "warning: {message}");
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;
    use crate::test_host::{TestHost, TestNode};

    const FRAME_TIME: f32 = 0.1;

    fn hero_sheet() -> SheetClips {
        let clip = |name: &str, frames: &[i32], looping| SheetClip {
            name: name.to_owned(),
            frames: frames.to_vec(),
            speed: 1.0 / FRAME_TIME,
            looping,
        };
        SheetClips {
            texture: "sprites/hero.png".to_owned(),
            columns: 4,
            rows: 2,
            clips: vec![
                clip("run", &[4, 5, 6], true),
                clip("attack", &[1, 2], false),
            ],
        }
    }

    fn hero() -> Rc<TestNode> {
        let reads = Rc::new(Cell::new(0));
        let counted = Rc::clone(&reads);
        set_loader(move |path| {
            counted.set(counted.get() + 1);
            match path {
                "hero.frames" => Ok(hero_sheet()),
                _ => Err("it does not exist".to_owned()),
            }
        });
        let sprite = SpriteFields::new("other.png".to_owned());
        sprite.frames.set("hero.frames".to_owned());
        Rc::new(TestNode {
            sprite: Some(sprite),
            ..TestNode::new("Hero")
        })
    }

    fn sprite(node: &TestNode) -> &SpriteFields<TestHost> {
        node.sprite.as_ref().expect("a sprite")
    }

    /// The frame shown after each of `frames` frames.
    fn frames_shown(node: &Rc<TestNode>, frames: usize) -> Vec<i32> {
        (0..frames)
            .map(|_| {
                advance::<TestHost>(node, FRAME_TIME);
                sprite(node).frame.get()
            })
            .collect()
    }

    #[test]
    fn sprites_take_the_image_and_cells_of_their_frames() {
        let hero = hero();

        advance::<TestHost>(&hero, FRAME_TIME);

        let look = sprite(&hero);
        assert_eq!(look.texture.get(), "sprites/hero.png");
        assert_eq!((look.columns.get(), look.rows.get()), (4, 2));
        assert!(!is_playing::<TestHost>(&hero));
    }

    #[test]
    fn looping_clips_go_round_their_frames() {
        let hero = hero();

        play::<TestHost>(&hero, "run");

        assert_eq!(frames_shown(&hero, 5), [4, 5, 6, 4, 5]);
        assert!(is_playing::<TestHost>(&hero));
        assert_eq!(animation::<TestHost>(&hero), "run");
    }

    #[test]
    fn clips_that_do_not_loop_stop_on_their_last_frame() {
        let hero = hero();

        play::<TestHost>(&hero, "attack");

        assert_eq!(frames_shown(&hero, 2), [1, 2]);
        assert!(is_playing::<TestHost>(&hero));
        assert_eq!(frames_shown(&hero, 2), [2, 2]);
        assert!(!is_playing::<TestHost>(&hero));
    }

    #[test]
    fn ended_clips_tell_the_sprite_once() {
        let hero = hero();
        play::<TestHost>(&hero, "attack");

        frames_shown(&hero, 2);
        assert_eq!(hero.calls(), Vec::<String>::new());
        frames_shown(&hero, 3);
        assert_eq!(hero.calls(), ["finished attack"]);

        play::<TestHost>(&hero, "run");
        frames_shown(&hero, 10);
        assert_eq!(hero.calls(), ["finished attack"]);
    }

    #[test]
    fn playing_the_clip_that_plays_does_not_restart_it() {
        let hero = hero();
        play::<TestHost>(&hero, "run");
        frames_shown(&hero, 2);

        play::<TestHost>(&hero, "run");
        assert_eq!(frames_shown(&hero, 1), [6]);

        play::<TestHost>(&hero, "attack");
        assert_eq!(frames_shown(&hero, 1), [1]);
    }

    #[test]
    fn stopped_clips_keep_their_frame_and_play_again_from_the_start() {
        let hero = hero();
        play::<TestHost>(&hero, "run");
        frames_shown(&hero, 2);

        stop::<TestHost>(&hero);
        assert_eq!(frames_shown(&hero, 2), [5, 5]);

        play::<TestHost>(&hero, "run");
        assert_eq!(frames_shown(&hero, 1), [4]);
    }

    #[test]
    fn missing_clips_and_files_stop_the_sprite_as_it_is() {
        let hero = hero();
        play::<TestHost>(&hero, "fly");

        assert_eq!(frames_shown(&hero, 1), [0]);
        assert!(!is_playing::<TestHost>(&hero));

        sprite(&hero).frames.set("missing.frames".to_owned());
        play::<TestHost>(&hero, "run");
        advance::<TestHost>(&hero, FRAME_TIME);
        assert_eq!(sprite(&hero).frame.get(), 0);
    }

    #[test]
    fn long_looping_clips_keep_their_time_short() {
        let clip = &hero_sheet().clips[0];

        let (index, time) = clip_position(clip, 0.35);
        assert_eq!(index, 0);
        assert!((time - 0.05).abs() < 1e-5, "{time}");

        let (_, time) = clip_position(clip, 1000.0);
        assert!(time < 0.3, "{time}");
    }
}
