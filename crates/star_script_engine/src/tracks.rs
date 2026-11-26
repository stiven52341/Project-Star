//! `.anim` files of the game's asset folder, read for the animators that
//! play their animations, and the animations of 3D models (`.glb`,
//! `.gltf`), which an animator plays on its parent `Model` (decision 83).

use star_engine::star_assets::{AssetFolder, AssetPath, ModelFormat};
use star_engine::star_scene::{
    Interpolation, MethodCall, PropertyTrack, TrackAnimation, TrackAnimations,
};
use star_script_runtime::tracks::{Ease, Track, TrackCall, TrackClip, TrackKey, Tracks};

use crate::scenes::scene_value;

/// Reads the `.anim` file at `path` of the asset `folder`.
///
/// # Errors
///
/// Why the file could not be read, for the warning the game writes.
pub fn read_animations(folder: &AssetFolder, path: &str) -> Result<Tracks, String> {
    if let Ok(model) = AssetPath::new(path)
        && ModelFormat::of(&model).is_ok()
    {
        return read_model_animations(folder, &model);
    }
    let file = AssetPath::new(path)
        .and_then(|path| folder.read_text(&path))
        .and_then(|text| TrackAnimations::from_text(&text))
        .map_err(|error| error.to_string())?;
    Ok(Tracks {
        animations: file.animations.into_iter().map(track_clip).collect(),
    })
}

/// The shortest a model's animation lasts: one with every key at the start
/// still plays.
const MIN_MODEL_ANIMATION_LENGTH: f32 = 0.001;

/// The animations of the model at `path`, as clips that loop and change no
/// property: the engine poses the model's skeleton with the times and
/// weights the animator shows.
fn read_model_animations(folder: &AssetFolder, path: &AssetPath) -> Result<Tracks, String> {
    let model = folder.load_model(path).map_err(|error| error.to_string())?;
    Ok(Tracks {
        animations: model
            .animations()
            .iter()
            .map(|animation| TrackClip {
                name: animation.name.clone(),
                length: animation.length.max(MIN_MODEL_ANIMATION_LENGTH),
                looping: true,
                tracks: Vec::new(),
                calls: Vec::new(),
            })
            .collect(),
    })
}

fn track_clip(animation: TrackAnimation) -> TrackClip {
    TrackClip {
        name: animation.name,
        length: animation.length,
        looping: animation.looping,
        tracks: animation.tracks.into_iter().map(track).collect(),
        calls: animation.calls.into_iter().map(track_call).collect(),
    }
}

fn track(track: PropertyTrack) -> Track {
    Track {
        node: track.node,
        property: track.property,
        ease: ease(track.interpolation),
        keys: track
            .keys
            .into_iter()
            .map(|key| TrackKey {
                time: key.time,
                value: scene_value(key.value),
            })
            .collect(),
    }
}

const fn ease(interpolation: Interpolation) -> Ease {
    match interpolation {
        Interpolation::Step => Ease::Step,
        Interpolation::Linear => Ease::Linear,
        Interpolation::EaseIn => Ease::In,
        Interpolation::EaseOut => Ease::Out,
        Interpolation::EaseInOut => Ease::InOut,
    }
}

fn track_call(call: MethodCall) -> TrackCall {
    TrackCall {
        node: call.node,
        time: call.time,
        method: call.method,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use star_script_runtime::scene_fields::SceneValue;

    use super::*;

    fn folder(name: &str, text: &str) -> AssetFolder {
        let root = std::env::temp_dir().join(format!("star_script_engine_tracks_{name}"));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("folder created");
        fs::write(root.join("knight.anim"), text).expect("animations written");
        AssetFolder::new(root)
    }

    #[test]
    fn animations_are_read_with_their_tracks_and_calls() {
        let text = "format = 1\n[[animation]]\nname = \"attack\"\nlength = 0.5\n\
                    [[animation.track]]\nnode = \"Arm\"\nproperty = \"rotation\"\n\
                    interpolation = \"ease_in_out\"\nkeys = [[0.0, 0.0], [0.5, 1]]\n\
                    [[animation.call]]\ntime = 0.25\nmethod = \"hit\"\n";

        let tracks = read_animations(&folder("read", text), "knight.anim").expect("read");

        let attack = &tracks.animations[0];
        assert_eq!((attack.name.as_str(), attack.length), ("attack", 0.5));
        assert!(!attack.looping);
        let arm = &attack.tracks[0];
        assert_eq!(arm.ease, Ease::InOut);
        assert_eq!(arm.keys[1].value, SceneValue::Int(1));
        assert_eq!(attack.calls[0].method, "hit");
    }

    #[test]
    fn broken_files_explain_why() {
        let error = read_animations(
            &folder("broken", "format = 1\n[[animation]]\n"),
            "knight.anim",
        )
        .unwrap_err();

        assert!(error.contains("animations file"), "{error}");
    }
}
