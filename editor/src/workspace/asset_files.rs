//! What the assets panel does to the files of the assets folder: creating
//! folders and files ready to use, renaming and deleting them. Paths are
//! inside the assets folder, with `/` between folders.

use std::fs;
use std::io;
use std::path::Path;

use star_engine::prelude::SceneOutline;

use super::project_classes::is_class_taken;
use crate::blueprint_editor::{BlueprintTemplate, new_blueprint_text, new_script_text};
use crate::frames_editor::new_frames_text;
use crate::project::{
    BLUEPRINT_EXTENSION, FRAMES_EXTENSION, KEYFRAMES_EXTENSION, MAX_NAME_LENGTH, ProjectDimension,
    RUST_EXTENSION, SCENE_EXTENSION, SCRIPT_EXTENSION, STATES_EXTENSION, add_missing_actions,
};

/// The scene new scenes start from: a root, placed in the project's space.
const NEW_SCENE_2D: &str = "format = 1\n[[node]]\nname = \"Root\"\nposition = [0.0, 0.0]\n";
const NEW_SCENE_3D: &str = "format = 1\n[[node]]\nname = \"Root\"\n[node.transform_3d]\n";
/// Characters a name may have besides letters and digits.
const NAME_SYMBOLS: [char; 3] = [' ', '-', '_'];

/// What the panel creates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NewAsset {
    Folder,
    Scene,
    Blueprint,
    Script,
    Rust,
    /// An animator's keyframe animations (`.anim`).
    Keyframes,
    /// An animator's state machine (`.states`).
    States,
    /// A copy of a ready-made blueprint.
    Template(BlueprintTemplate),
}

impl NewAsset {
    /// Every kind, in the order the menu lists them.
    pub(super) const ALL: [Self; 7] = [
        Self::Folder,
        Self::Scene,
        Self::Blueprint,
        Self::Script,
        Self::Rust,
        Self::Keyframes,
        Self::States,
    ];

    /// The name a new one is given until the user changes it.
    pub(super) const fn default_name(self) -> &'static str {
        match self {
            Self::Folder => "New Folder",
            Self::Scene => "NewScene",
            Self::Blueprint => "NewBlueprint",
            Self::Script => "NewScript",
            Self::Rust => "new_module",
            Self::Keyframes => "NewAnimations",
            Self::States => "NewStates",
            Self::Template(template) => template.name(),
        }
    }

    /// Whether it declares a class named as its file.
    const fn declares_class(self) -> bool {
        matches!(self, Self::Blueprint | Self::Script | Self::Template(_))
    }

    const fn extension(self) -> &'static str {
        match self {
            Self::Folder => "",
            Self::Scene => SCENE_EXTENSION,
            Self::Blueprint | Self::Template(_) => BLUEPRINT_EXTENSION,
            Self::Script => SCRIPT_EXTENSION,
            Self::Rust => RUST_EXTENSION,
            Self::Keyframes => KEYFRAMES_EXTENSION,
            Self::States => STATES_EXTENSION,
        }
    }
}

/// Why a file could not be created, renamed or deleted.
#[derive(Debug)]
pub(crate) enum AssetFileError {
    /// The name has characters files cannot have, or is empty or long.
    InvalidName,
    /// Classes and modules are named as code is: letters, digits and `_`,
    /// not starting with a digit.
    InvalidCodeName,
    /// Something else is already called that.
    AlreadyExists(String),
    /// A script or blueprint would declare a class the engine or another
    /// script or blueprint already declares: they form one program.
    ClassExists(String),
    /// The disk refused.
    Io(io::Error),
}

impl From<io::Error> for AssetFileError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// Creates a new `kind` called `name` in `folder` of the `assets` folder,
/// for a project of `dimension`, and returns its path. It never
/// overwrites anything.
///
/// # Errors
///
/// Fails if the name is not valid, something is already called so (a
/// file, or the class of a script or blueprint), or the disk refuses.
pub(super) fn create(
    assets: &Path,
    folder: &str,
    kind: NewAsset,
    name: &str,
    dimension: ProjectDimension,
) -> Result<String, AssetFileError> {
    let name = name.trim();
    let stem = name.strip_suffix(kind.extension()).unwrap_or(name);
    check_name(stem)?;
    let path = join(folder, &format!("{stem}{}", kind.extension()));
    let target = assets.join(&path);
    if target.exists() {
        return Err(AssetFileError::AlreadyExists(path));
    }
    if kind.declares_class() && is_class_taken(assets, stem, None) {
        return Err(AssetFileError::ClassExists(stem.to_owned()));
    }
    if kind == NewAsset::Folder {
        fs::create_dir_all(&target)?;
        return Ok(path);
    }
    write_new(&target, &contents(kind, stem, dimension)?)?;
    if let NewAsset::Template(template) = kind {
        add_template_actions(assets, template);
    }
    Ok(path)
}

/// Creates a `.frames` file called `name` in `folder` of the `assets`
/// folder, of the image at `image` (whole, without clips), and returns its
/// path. It never overwrites anything.
///
/// # Errors
///
/// Fails if the name is not valid, a file is already called so, the
/// image's path is not an asset path, or the disk refuses.
pub(super) fn create_frames(
    assets: &Path,
    folder: &str,
    name: &str,
    image: &str,
) -> Result<String, AssetFileError> {
    let name = name.trim();
    let stem = name.strip_suffix(FRAMES_EXTENSION).unwrap_or(name);
    check_name(stem)?;
    let path = join(folder, &format!("{stem}{FRAMES_EXTENSION}"));
    let target = assets.join(&path);
    if target.exists() {
        return Err(AssetFileError::AlreadyExists(path));
    }
    let contents = new_frames_text(image).map_err(io::Error::other)?;
    write_new(&target, &contents)?;
    Ok(path)
}

/// Writes `contents` to a new file at `target`, making its folder if
/// needed; fails rather than overwrite a file.
fn write_new(target: &Path, contents: &str) -> io::Result<()> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(target)?;
    io::Write::write_all(&mut file, contents.as_bytes())
}

/// Adds to the project the input actions `template` reads, if it lacks
/// them. The blueprint is made either way, so a failure is only logged:
/// “Compile” then names the actions missing.
fn add_template_actions(assets: &Path, template: BlueprintTemplate) {
    let Some(project) = assets.parent() else {
        return;
    };
    match add_missing_actions(project, template.actions()) {
        Ok(added) if !added.is_empty() => {
            log::info!(
                "Added the input actions {} to project.toml",
                added.join(", ")
            );
        }
        Ok(_) => {}
        Err(error) => log::warn!(
            "Cannot add the input actions of {}: {error}",
            template.name()
        ),
    }
}

/// What a new `kind` called `stem` starts with.
fn contents(
    kind: NewAsset,
    stem: &str,
    dimension: ProjectDimension,
) -> Result<String, AssetFileError> {
    match kind {
        NewAsset::Folder => Ok(String::new()),
        NewAsset::Scene => new_scene(stem, dimension),
        NewAsset::Blueprint => new_blueprint_text(stem).ok_or(AssetFileError::InvalidCodeName),
        NewAsset::Script => new_script_text(stem).ok_or(AssetFileError::InvalidCodeName),
        NewAsset::Template(template) => {
            if !is_code_name(stem) {
                return Err(AssetFileError::InvalidCodeName);
            }
            template
                .text(stem)
                .ok_or_else(|| AssetFileError::Io(io::Error::other("unreadable template")))
        }
        NewAsset::Rust => {
            if !is_code_name(stem) {
                return Err(AssetFileError::InvalidCodeName);
            }
            Ok(format!("//! {stem}: Rust code of the game.\n"))
        }
        NewAsset::Keyframes => Ok(NEW_KEYFRAMES.to_owned()),
        NewAsset::States => Ok(NEW_STATES.to_owned()),
    }
}

/// A new `.anim` file: no animations yet, and an example of one to write,
/// until the editor has a timeline for them.
const NEW_KEYFRAMES: &str = r#"format = 1

# Keyframe animations, which an Animator plays on its parent and the nodes
# under it. Write them as in this example (without the `#`):
#
# [[animation]]
# name = "idle"
# length = 1.0
# looping = true
#
# [[animation.track]]
# node = ""                      # "" is the animator's parent; "Arm" its child
# property = "y"
# interpolation = "ease_in_out"  # step, linear, ease_in, ease_out, ease_in_out
# keys = [[0.0, 0.0], [0.5, 0.1], [1.0, 0.0]]
#
# [[animation.call]]
# time = 0.5
# method = "step"
"#;

/// A new `.states` file: a machine with one state, playing the animation
/// `idle`, to grow in its graph.
const NEW_STATES: &str = r#"format = 1
start = "idle"

[[state]]
name = "idle"
animation = "idle"
position = [0.0, 0.0]
"#;

/// A scene with only its root, named `stem`.
fn new_scene(stem: &str, dimension: ProjectDimension) -> Result<String, AssetFileError> {
    let text = match dimension {
        ProjectDimension::TwoD => NEW_SCENE_2D,
        ProjectDimension::ThreeD => NEW_SCENE_3D,
    };
    let mut outline = SceneOutline::from_text(text).map_err(io::Error::other)?;
    if let Some(root) = outline.node_mut(0) {
        stem.clone_into(&mut root.name);
    }
    outline
        .to_text()
        .map_err(|error| AssetFileError::Io(io::Error::other(error)))
}

/// Renames the file or folder at `path` to `name`, keeping a file's
/// extension, and returns its new path.
///
/// # Errors
///
/// Fails if the name is not valid, something is already called so, or the
/// disk refuses.
pub(super) fn rename(
    assets: &Path,
    path: &str,
    name: &str,
    is_folder: bool,
) -> Result<String, AssetFileError> {
    let (folder, old_name) = path.rsplit_once('/').unwrap_or(("", path));
    let extension = match old_name.rsplit_once('.') {
        Some((_, extension)) if !is_folder => format!(".{extension}"),
        _ => String::new(),
    };
    let name = name.trim();
    let stem = name.strip_suffix(extension.as_str()).unwrap_or(name);
    check_name(stem)?;
    let new_path = join(folder, &format!("{stem}{extension}"));
    if new_path == path {
        return Ok(new_path);
    }
    let target = assets.join(&new_path);
    if target.exists() {
        return Err(AssetFileError::AlreadyExists(new_path));
    }
    // Renaming a script or blueprint renames the class called as it.
    let is_class_file =
        !is_folder && (path.ends_with(SCRIPT_EXTENSION) || path.ends_with(BLUEPRINT_EXTENSION));
    if is_class_file && is_class_taken(assets, stem, Some(path)) {
        return Err(AssetFileError::ClassExists(stem.to_owned()));
    }
    fs::rename(assets.join(path), target)?;
    Ok(new_path)
}

/// Deletes the file or folder (with everything in it) at `path`.
///
/// # Errors
///
/// Fails if the disk refuses.
pub(super) fn delete(assets: &Path, path: &str, is_folder: bool) -> io::Result<()> {
    let target = assets.join(path);
    if is_folder {
        fs::remove_dir_all(target)
    } else {
        fs::remove_file(target)
    }
}

/// The name of a file or folder without its extension, inside `folder`.
fn join(folder: &str, name: &str) -> String {
    if folder.is_empty() {
        name.to_owned()
    } else {
        format!("{folder}/{name}")
    }
}

/// Accepts names of letters, digits, spaces, `-` and `_`, starting with a
/// letter, a digit or `_`: names every system can store.
fn check_name(stem: &str) -> Result<(), AssetFileError> {
    let mut characters = stem.chars();
    let starts_well = characters
        .next()
        .is_some_and(|first| first.is_alphanumeric() || first == '_');
    let rest_is_valid = stem
        .chars()
        .all(|character| character.is_alphanumeric() || NAME_SYMBOLS.contains(&character));
    if starts_well && rest_is_valid && stem.chars().count() <= MAX_NAME_LENGTH {
        Ok(())
    } else {
        Err(AssetFileError::InvalidName)
    }
}

/// Whether `stem` can name a Rust module: letters, digits and `_`, not
/// starting with a digit.
fn is_code_name(stem: &str) -> bool {
    stem.chars()
        .next()
        .is_some_and(|first| first.is_alphabetic() || first == '_')
        && stem
            .chars()
            .all(|character| character.is_alphanumeric() || character == '_')
}

#[cfg(test)]
mod tests {
    use star_engine::star_scene::TrackAnimations;

    use super::*;
    use crate::project::TestDirectory;

    fn assets(name: &str) -> TestDirectory {
        let assets = TestDirectory::new(name);
        fs::create_dir_all(assets.path().join("levels")).unwrap();
        assets
    }

    fn create_in(assets: &TestDirectory, kind: NewAsset, name: &str) -> String {
        create(assets.path(), "levels", kind, name, ProjectDimension::TwoD).unwrap()
    }

    #[test]
    fn new_files_get_their_extension_and_contents() {
        let assets = assets("asset_files_create");

        let scene = create_in(&assets, NewAsset::Scene, "Intro");
        let script = create_in(&assets, NewAsset::Script, "Rock.star");
        let folder = create_in(&assets, NewAsset::Folder, "Bosses");

        assert_eq!(scene, "levels/Intro.scene");
        assert_eq!(script, "levels/Rock.star");
        assert!(assets.path().join(folder).is_dir());
        let text = fs::read_to_string(assets.path().join(&scene)).unwrap();
        let outline = SceneOutline::from_text(&text).unwrap();
        assert_eq!(outline.nodes()[0].name, "Intro");
        assert!(outline.nodes()[0].transform.is_some());
    }

    #[test]
    fn animations_are_made_of_an_image_and_never_overwrite() {
        let assets = assets("asset_files_frames");

        let path = create_frames(assets.path(), "levels", "hero", "sprites/hero.png").unwrap();

        assert_eq!(path, "levels/hero.frames");
        let text = fs::read_to_string(assets.path().join(&path)).unwrap();
        let frames = star_engine::star_scene::SpriteFrames::from_text(&text).unwrap();
        assert_eq!(frames.texture.as_str(), "sprites/hero.png");
        assert!(matches!(
            create_frames(assets.path(), "levels", "hero.frames", "sprites/hero.png"),
            Err(AssetFileError::AlreadyExists(_))
        ));
    }

    #[test]
    fn every_kind_can_be_created() {
        let assets = assets("asset_files_every_kind");

        for kind in NewAsset::ALL {
            let path = create_in(&assets, kind, kind.default_name());
            assert!(assets.path().join(path).exists());
        }
    }

    #[test]
    fn new_keyframe_animations_are_valid_and_empty() {
        let assets = assets("asset_files_keyframes");

        let path = create_in(&assets, NewAsset::Keyframes, "Knight");

        assert!(path.ends_with("Knight.anim"), "{path}");
        let text = fs::read_to_string(assets.path().join(path)).unwrap();
        let animations = TrackAnimations::from_text(&text).unwrap();
        assert_eq!(animations, TrackAnimations::default());
    }

    #[test]
    fn templates_are_copied_with_their_name_and_bring_their_actions() {
        let project = TestDirectory::new("asset_files_template");
        let assets = project.path().join("assets");
        fs::create_dir_all(&assets).unwrap();
        fs::write(project.path().join("project.toml"), "[project]\n").unwrap();

        let template = NewAsset::Template(BlueprintTemplate::Movement2D);
        let path = create(&assets, "", template, "Player", ProjectDimension::TwoD).unwrap();

        assert_eq!(path, "Player.blueprint");
        let text = fs::read_to_string(assets.join(&path)).unwrap();
        assert!(text.contains("class = \"Player\""), "{text}");
        let manifest = fs::read_to_string(project.path().join("project.toml")).unwrap();
        assert!(
            manifest.contains("left = [\"A\", \"ArrowLeft\""),
            "{manifest}"
        );
    }

    // Regression: a script and a blueprint could both declare
    // `StarMovement`, and the project only failed when it played.
    #[test]
    fn scripts_and_blueprints_need_a_class_name_of_their_own() {
        let assets = assets("asset_files_class_taken");
        create_in(&assets, NewAsset::Blueprint, "StarMovement");
        let in_root = |kind, name| create(assets.path(), "", kind, name, ProjectDimension::TwoD);

        let script = in_root(NewAsset::Script, "StarMovement");
        let engine = in_root(NewAsset::Blueprint, "Sprite");
        let template = in_root(NewAsset::Template(BlueprintTemplate::Spinner2D), "Sprite");

        for result in [script, engine, template] {
            assert!(
                matches!(&result, Err(AssetFileError::ClassExists(_))),
                "{result:?}"
            );
        }
        assert!(!assets.path().join("StarMovement.star").exists());
        assert!(in_root(NewAsset::Scene, "StarMovement").is_ok());
        assert!(in_root(NewAsset::Script, "StarSpin").is_ok());
    }

    #[test]
    fn scripts_are_not_renamed_after_a_class_that_exists() {
        let assets = assets("asset_files_rename_class_taken");
        let rock = create_in(&assets, NewAsset::Script, "Rock");
        create_in(&assets, NewAsset::Blueprint, "Ship");

        let result = rename(assets.path(), &rock, "Ship", false);

        assert!(
            matches!(result, Err(AssetFileError::ClassExists(_))),
            "{result:?}"
        );
        assert!(assets.path().join(&rock).exists());
        assert_eq!(
            rename(assets.path(), &rock, "Stone", false).unwrap(),
            "levels/Stone.star"
        );
    }

    #[test]
    fn nothing_is_overwritten() {
        let assets = assets("asset_files_exists");
        fs::write(assets.path().join("levels/Rock.blueprint"), "mine").unwrap();

        let result = create(
            assets.path(),
            "levels",
            NewAsset::Blueprint,
            "Rock",
            ProjectDimension::TwoD,
        );

        assert!(matches!(result, Err(AssetFileError::AlreadyExists(_))));
        let text = fs::read_to_string(assets.path().join("levels/Rock.blueprint")).unwrap();
        assert_eq!(text, "mine");
    }

    #[test]
    fn names_are_checked() {
        let assets = assets("asset_files_names");
        let try_create =
            |kind, name| create(assets.path(), "", kind, name, ProjectDimension::TwoD).map(|_| ());

        assert!(matches!(
            try_create(NewAsset::Folder, "../out"),
            Err(AssetFileError::InvalidName)
        ));
        assert!(matches!(
            try_create(NewAsset::Scene, ""),
            Err(AssetFileError::InvalidName)
        ));
        assert!(matches!(
            try_create(NewAsset::Script, "Big Rock"),
            Err(AssetFileError::InvalidCodeName)
        ));
        assert!(matches!(
            try_create(NewAsset::Rust, "2d"),
            Err(AssetFileError::InvalidCodeName)
        ));
    }

    #[test]
    fn renaming_keeps_the_extension_and_the_folder() {
        let assets = assets("asset_files_rename");
        let scene = create_in(&assets, NewAsset::Scene, "Intro");

        let renamed = rename(assets.path(), &scene, "Start", false).unwrap();

        assert_eq!(renamed, "levels/Start.scene");
        assert!(assets.path().join(&renamed).exists());
        assert!(!assets.path().join(&scene).exists());
    }

    #[test]
    fn renaming_onto_another_file_is_refused() {
        let assets = assets("asset_files_rename_taken");
        let first = create_in(&assets, NewAsset::Scene, "One");
        create_in(&assets, NewAsset::Scene, "Two");

        let result = rename(assets.path(), &first, "Two", false);

        assert!(matches!(result, Err(AssetFileError::AlreadyExists(_))));
    }

    #[test]
    fn folders_are_deleted_with_their_contents() {
        let assets = assets("asset_files_delete");
        create_in(&assets, NewAsset::Scene, "Intro");

        delete(assets.path(), "levels", true).unwrap();

        assert!(!assets.path().join("levels").exists());
    }
}
