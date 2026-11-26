//! `.frames` files of the game's asset folder, read for the sprites that
//! play their clips.

use star_engine::star_assets::{AssetFolder, AssetPath};
use star_engine::star_scene::{FrameClip, SpriteFrames};
use star_script_runtime::animations::{SheetClip, SheetClips};

/// Reads the `.frames` file at `path` of the asset `folder`.
///
/// # Errors
///
/// Why the file could not be read, for the warning the game writes.
pub fn read_frames(folder: &AssetFolder, path: &str) -> Result<SheetClips, String> {
    let frames = AssetPath::new(path)
        .and_then(|path| folder.read_text(&path))
        .and_then(|text| SpriteFrames::from_text(&text))
        .map_err(|error| error.to_string())?;
    Ok(SheetClips {
        texture: frames.texture.as_str().to_owned(),
        columns: i32::from(frames.columns),
        rows: i32::from(frames.rows),
        clips: frames.clips.iter().map(sheet_clip).collect(),
    })
}

/// A clip with the words and values of StarScript's `Sprite`.
fn sheet_clip(clip: &FrameClip) -> SheetClip {
    SheetClip {
        name: clip.name.clone(),
        // A valid sheet has at most `u16::MAX` squared cells, past `i32`
        // only in sheets no image could hold.
        frames: clip
            .frames
            .iter()
            .map(|&frame| i32::try_from(frame).unwrap_or(i32::MAX))
            .collect(),
        speed: clip.speed,
        looping: clip.looping,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn folder(name: &str, text: &str) -> AssetFolder {
        let root = std::env::temp_dir().join(format!("star_script_engine_frames_{name}"));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("folder created");
        fs::write(root.join("hero.frames"), text).expect("frames written");
        AssetFolder::new(root)
    }

    #[test]
    fn frames_are_read_with_their_clips() {
        let text = "format = 1\ntexture = \"hero.png\"\ncolumns = 4\nrows = 2\n\
                    [[clip]]\nname = \"run\"\nframes = [4, 5]\nspeed = 8.0\nlooping = false\n";

        let sheet = read_frames(&folder("read", text), "hero.frames").expect("read");

        assert_eq!(sheet.texture, "hero.png");
        assert_eq!((sheet.columns, sheet.rows), (4, 2));
        assert_eq!(
            sheet.clips,
            [SheetClip {
                name: "run".to_owned(),
                frames: vec![4, 5],
                speed: 8.0,
                looping: false,
            }]
        );
    }

    #[test]
    fn broken_or_missing_frames_are_explained() {
        let folder = folder("broken", "format = 1\ntexture = \"hero.png\"\nrows = 0\n");

        let broken = read_frames(&folder, "hero.frames").unwrap_err();
        let missing = read_frames(&folder, "nobody.frames").unwrap_err();

        assert!(broken.contains("one row"), "{broken}");
        assert!(missing.contains("nobody.frames"), "{missing}");
    }
}
