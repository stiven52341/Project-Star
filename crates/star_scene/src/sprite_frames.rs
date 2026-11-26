//! `.frames` files: the animations of a sprite sheet, as named clips of its
//! cells, that sprites play (decision 78).

use serde::{Deserialize, Serialize};
use star_assets::AssetPath;
use star_core::{EngineError, EngineResult};

/// The version of the layout this engine writes and reads.
const FORMAT_VERSION: u32 = 1;

/// The frames per second of a clip that does not say.
pub const DEFAULT_CLIP_SPEED: f32 = 10.0;

/// The extension of the files, with its dot.
pub const FRAMES_EXTENSION: &str = ".frames";

/// The animations of a sprite sheet: its image, how it is cut into cells,
/// and clips of those cells.
#[derive(Debug, Clone, PartialEq)]
pub struct SpriteFrames {
    /// The image, in the asset folder.
    pub texture: AssetPath,
    /// How many cells wide the image is.
    pub columns: u16,
    /// How many cells tall the image is.
    pub rows: u16,
    /// The animations, each with a different name.
    pub clips: Vec<FrameClip>,
}

/// One animation of a [`SpriteFrames`]: cells shown one after another.
#[derive(Debug, Clone, PartialEq)]
pub struct FrameClip {
    /// What scripts play it by (`"run"`).
    pub name: String,
    /// The cells shown, in order, counted from the top-left one, row by
    /// row. Never empty.
    pub frames: Vec<u32>,
    /// Frames shown per second; above zero.
    pub speed: f32,
    /// Whether it starts over after its last frame, or stays on it.
    pub looping: bool,
}

impl SpriteFrames {
    /// The animations of the image `texture`, whole, with no clips yet.
    #[must_use]
    pub const fn new(texture: AssetPath) -> Self {
        Self {
            texture,
            columns: 1,
            rows: 1,
            clips: Vec::new(),
        }
    }

    /// Reads the text of a `.frames` file.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Asset`] if it is not valid TOML of this
    /// layout, or its values do not make a valid sheet (see
    /// [`SpriteFrames::validate`]).
    pub fn from_text(text: &str) -> EngineResult<Self> {
        let file: FramesFile = toml::from_str(text)
            .map_err(|error| invalid(&format!("invalid frames file: {error}")))?;
        if file.format != FORMAT_VERSION {
            return Err(invalid(&format!(
                "the frames file has format {}, but this engine reads format {FORMAT_VERSION}",
                file.format
            )));
        }
        let frames = Self {
            texture: AssetPath::new(&file.texture)?,
            columns: file.columns,
            rows: file.rows,
            clips: file.clips.into_iter().map(FrameClip::from).collect(),
        };
        frames.validate()?;
        Ok(frames)
    }

    /// Writes the text of a `.frames` file.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Asset`] if its values do not make a valid
    /// sheet (see [`SpriteFrames::validate`]).
    pub fn to_text(&self) -> EngineResult<String> {
        self.validate()?;
        let file = FramesFile {
            format: FORMAT_VERSION,
            texture: self.texture.as_str().to_owned(),
            columns: self.columns,
            rows: self.rows,
            clips: self.clips.iter().map(ClipData::from).collect(),
        };
        toml::to_string(&file)
            .map_err(|error| invalid(&format!("cannot write the frames file: {error}")))
    }

    /// The clip called `name`.
    #[must_use]
    pub fn clip(&self, name: &str) -> Option<&FrameClip> {
        self.clips.iter().find(|clip| clip.name == name)
    }

    /// How many cells the image is cut into.
    #[must_use]
    pub fn cells(&self) -> u32 {
        u32::from(self.columns) * u32::from(self.rows)
    }

    /// Checks that the sheet has at least one column and row, and that
    /// every clip has a name of its own, at least one frame, only cells of
    /// the sheet and a speed above zero.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Asset`] naming the first value that is not.
    pub fn validate(&self) -> EngineResult<()> {
        if self.columns == 0 || self.rows == 0 {
            return Err(invalid("a sheet needs at least one column and one row"));
        }
        for (index, clip) in self.clips.iter().enumerate() {
            self.validate_clip(clip)?;
            if self.clips[..index]
                .iter()
                .any(|other| other.name == clip.name)
            {
                return Err(invalid(&format!("two clips are called \"{}\"", clip.name)));
            }
        }
        Ok(())
    }

    fn validate_clip(&self, clip: &FrameClip) -> EngineResult<()> {
        let name = &clip.name;
        if name.trim().is_empty() {
            return Err(invalid("a clip has no name"));
        }
        if clip.frames.is_empty() {
            return Err(invalid(&format!("the clip \"{name}\" has no frames")));
        }
        if let Some(frame) = clip.frames.iter().find(|&&frame| frame >= self.cells()) {
            return Err(invalid(&format!(
                "the clip \"{name}\" shows the frame {frame}, but the sheet has {} cells",
                self.cells()
            )));
        }
        if !(clip.speed.is_finite() && clip.speed > 0.0) {
            return Err(invalid(&format!(
                "the clip \"{name}\" needs a speed above zero"
            )));
        }
        Ok(())
    }
}

fn invalid(reason: &str) -> EngineError {
    EngineError::Asset(reason.to_owned())
}

/// A whole `.frames` file.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FramesFile {
    format: u32,
    texture: String,
    #[serde(default = "one")]
    columns: u16,
    #[serde(default = "one")]
    rows: u16,
    #[serde(rename = "clip", default)]
    clips: Vec<ClipData>,
}

/// One `[[clip]]` table.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ClipData {
    name: String,
    frames: Vec<u32>,
    #[serde(default = "default_speed")]
    speed: f32,
    #[serde(default = "yes")]
    looping: bool,
}

const fn one() -> u16 {
    1
}

const fn default_speed() -> f32 {
    DEFAULT_CLIP_SPEED
}

const fn yes() -> bool {
    true
}

impl From<ClipData> for FrameClip {
    fn from(clip: ClipData) -> Self {
        Self {
            name: clip.name,
            frames: clip.frames,
            speed: clip.speed,
            looping: clip.looping,
        }
    }
}

impl From<&FrameClip> for ClipData {
    fn from(clip: &FrameClip) -> Self {
        Self {
            name: clip.name.clone(),
            frames: clip.frames.clone(),
            speed: clip.speed,
            looping: clip.looping,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HERO: &str = r#"
        format = 1
        texture = "sprites/hero.png"
        columns = 8
        rows = 4

        [[clip]]
        name = "run"
        frames = [8, 9, 10, 11, 12, 13]
        speed = 12.0
        looping = true

        [[clip]]
        name = "attack"
        frames = [16, 17, 18]
        looping = false
    "#;

    fn error_of(text: &str) -> String {
        SpriteFrames::from_text(text).unwrap_err().to_string()
    }

    #[test]
    fn files_are_read_with_their_clips() {
        let frames = SpriteFrames::from_text(HERO).unwrap();

        assert_eq!(frames.texture.as_str(), "sprites/hero.png");
        assert_eq!((frames.columns, frames.rows, frames.cells()), (8, 4, 32));
        let run = frames.clip("run").unwrap();
        assert_eq!(run.frames, [8, 9, 10, 11, 12, 13]);
        assert!(run.looping);
        let attack = frames.clip("attack").unwrap();
        assert_eq!(attack.speed, DEFAULT_CLIP_SPEED);
        assert!(!attack.looping);
        assert!(frames.clip("jump").is_none());
    }

    #[test]
    fn written_files_read_back_the_same() {
        let frames = SpriteFrames::from_text(HERO).unwrap();

        let written = frames.to_text().unwrap();

        assert_eq!(SpriteFrames::from_text(&written).unwrap(), frames);
    }

    #[test]
    fn a_whole_image_without_clips_is_a_valid_sheet() {
        let path = AssetPath::new("hero.png").unwrap();
        let frames = SpriteFrames::new(path);

        let text = frames.to_text().unwrap();

        assert_eq!(SpriteFrames::from_text(&text).unwrap(), frames);
    }

    #[test]
    fn clips_must_show_cells_of_the_sheet() {
        let text = "format = 1\ntexture = \"a.png\"\ncolumns = 2\n\
                    [[clip]]\nname = \"run\"\nframes = [0, 2]\n";

        assert!(error_of(text).contains("the clip \"run\" shows the frame 2"));
    }

    #[test]
    fn clips_need_frames_a_speed_and_a_name_of_their_own() {
        let clip = |name: &str, frames: &str, speed: &str| {
            format!(
                "format = 1\ntexture = \"a.png\"\ncolumns = 4\n\
                 [[clip]]\nname = \"{name}\"\nframes = {frames}\nspeed = {speed}\n"
            )
        };
        assert!(error_of(&clip("run", "[]", "1.0")).contains("has no frames"));
        assert!(error_of(&clip("run", "[0]", "0.0")).contains("speed above zero"));
        assert!(error_of(&clip(" ", "[0]", "1.0")).contains("has no name"));
        let twice = format!(
            "{}[[clip]]\nname = \"run\"\nframes = [1]\n",
            clip("run", "[0]", "1.0")
        );
        assert!(error_of(&twice).contains("two clips are called \"run\""));
    }

    #[test]
    fn other_formats_and_empty_sheets_are_refused() {
        assert!(error_of("format = 2\ntexture = \"a.png\"").contains("format 2"));
        assert!(error_of("format = 1\ntexture = \"a.png\"\nrows = 0").contains("one row"));
        assert!(error_of("format = 1\ntexture = \"../a.png\"").contains("asset"));
    }
}
