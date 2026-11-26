//! `.anim` files: animations of a node and the nodes under it, as tracks of
//! keys over time, that an `Animator` plays.
//!
//! A property track changes one property of one node (its `x`, its
//! `color`, an `export` field of its class) from key to key; a call calls a
//! method of a node when the animation passes its time. Nodes are named by
//! their path from the node animated: `""` is that node, `"Arm/Hand"` the
//! child `Hand` of its child `Arm`.

use serde::{Deserialize, Serialize};
use star_core::{EngineError, EngineResult};

use crate::FieldValue;

/// The version of the layout this engine writes and reads.
const FORMAT_VERSION: u32 = 1;

/// The extension of the files, with its dot.
pub const ANIMATIONS_EXTENSION: &str = ".anim";

/// The separator of the names in a node's path.
pub const PATH_SEPARATOR: char = '/';

/// The animations of an `.anim` file.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TrackAnimations {
    /// The animations, each with a different name.
    pub animations: Vec<TrackAnimation>,
}

/// One animation: tracks that play together for its length.
#[derive(Debug, Clone, PartialEq)]
pub struct TrackAnimation {
    /// What scripts play it by (`"attack"`).
    pub name: String,
    /// How long it lasts, in seconds; above zero.
    pub length: f32,
    /// Whether it starts over at its end, or stays there.
    pub looping: bool,
    /// The properties it changes; never two for the same property of the
    /// same node.
    pub tracks: Vec<PropertyTrack>,
    /// The methods it calls.
    pub calls: Vec<MethodCall>,
}

/// A property of a node, changed from key to key.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyTrack {
    /// The node's path from the node animated (see the module's
    /// documentation).
    pub node: String,
    /// The property: a field of the node's class (`"x"`, `"color"`, an
    /// `export` field).
    pub property: String,
    /// How the value goes from one key to the next.
    pub interpolation: Interpolation,
    /// The keys, by time; never empty, all with values of the same kind
    /// (whole numbers and decimals count as one).
    pub keys: Vec<Key>,
}

/// The value a [`PropertyTrack`] has at a time.
#[derive(Debug, Clone, PartialEq)]
pub struct Key {
    /// Seconds from the start of the animation, within its length.
    pub time: f32,
    /// The value then.
    pub value: FieldValue,
}

/// How a [`PropertyTrack`] goes from a key to the next. Decimals and
/// colors follow it; whole numbers, `bool`s and texts always jump.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Interpolation {
    /// Keeps a key's value until the next key.
    Step,
    /// Changes at the same pace from key to key.
    #[default]
    Linear,
    /// Starts slowly and speeds up.
    EaseIn,
    /// Starts fast and slows down.
    EaseOut,
    /// Starts and ends slowly.
    EaseInOut,
}

/// A method called when the animation passes a time.
#[derive(Debug, Clone, PartialEq)]
pub struct MethodCall {
    /// The node's path from the node animated (see the module's
    /// documentation).
    pub node: String,
    /// Seconds from the start of the animation, within its length.
    pub time: f32,
    /// The method, which takes no arguments.
    pub method: String,
}

impl TrackAnimations {
    /// Reads the text of an `.anim` file.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Asset`] if it is not valid TOML of this
    /// layout, or its values do not make valid animations (see
    /// [`TrackAnimations::validate`]).
    pub fn from_text(text: &str) -> EngineResult<Self> {
        let file: AnimationsFile = toml::from_str(text)
            .map_err(|error| invalid(&format!("invalid animations file: {error}")))?;
        if file.format != FORMAT_VERSION {
            return Err(invalid(&format!(
                "the animations file has format {}, but this engine reads format {FORMAT_VERSION}",
                file.format
            )));
        }
        let animations = Self {
            animations: file
                .animations
                .into_iter()
                .map(TrackAnimation::from)
                .collect(),
        };
        animations.validate()?;
        Ok(animations)
    }

    /// Writes the text of an `.anim` file.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Asset`] if its values do not make valid
    /// animations (see [`TrackAnimations::validate`]).
    pub fn to_text(&self) -> EngineResult<String> {
        self.validate()?;
        let file = AnimationsFile {
            format: FORMAT_VERSION,
            animations: self.animations.iter().map(AnimationData::from).collect(),
        };
        toml::to_string(&file)
            .map_err(|error| invalid(&format!("cannot write the animations file: {error}")))
    }

    /// The animation called `name`.
    #[must_use]
    pub fn animation(&self, name: &str) -> Option<&TrackAnimation> {
        self.animations
            .iter()
            .find(|animation| animation.name == name)
    }

    /// Checks that every animation has a name of its own, a length above
    /// zero, and valid tracks and calls: valid node paths, a property or
    /// method named, times within the length (keys in order, never two at
    /// once), keys of one kind of value, and one track per property.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Asset`] naming the first value that is not.
    pub fn validate(&self) -> EngineResult<()> {
        for (index, animation) in self.animations.iter().enumerate() {
            animation.validate()?;
            if self.animations[..index]
                .iter()
                .any(|other| other.name == animation.name)
            {
                return Err(invalid(&format!(
                    "two animations are called \"{}\"",
                    animation.name
                )));
            }
        }
        Ok(())
    }
}

impl TrackAnimation {
    fn validate(&self) -> EngineResult<()> {
        let name = &self.name;
        if name.trim().is_empty() {
            return Err(invalid("an animation has no name"));
        }
        if !(self.length.is_finite() && self.length > 0.0) {
            return Err(invalid(&format!(
                "the animation \"{name}\" needs a length above zero"
            )));
        }
        let within = |reason: String| invalid(&format!("the animation \"{name}\": {reason}"));
        for (index, track) in self.tracks.iter().enumerate() {
            track.validate(self.length).map_err(within)?;
            if self.tracks[..index]
                .iter()
                .any(|other| other.node == track.node && other.property == track.property)
            {
                return Err(within(format!(
                    "two tracks change `{}` of {}",
                    track.property,
                    node_name(&track.node)
                )));
            }
        }
        for call in &self.calls {
            call.validate(self.length).map_err(within)?;
        }
        Ok(())
    }
}

impl PropertyTrack {
    fn validate(&self, length: f32) -> Result<(), String> {
        validate_path(&self.node)?;
        let node = node_name(&self.node);
        let property = &self.property;
        if property.trim().is_empty() {
            return Err(format!("a track of {node} names no property"));
        }
        let track = format!("the track of `{property}` of {node}");
        let Some(first) = self.keys.first() else {
            return Err(format!("{track} has no keys"));
        };
        let mut previous: Option<f32> = None;
        for key in &self.keys {
            if !within_length(key.time, length) {
                return Err(format!(
                    "{track} has a key at {}, outside the animation's {length} seconds",
                    key.time
                ));
            }
            if previous.is_some_and(|previous| key.time <= previous) {
                return Err(format!("{track} has keys out of order or at the same time"));
            }
            previous = Some(key.time);
            if !same_kind(&first.value, &key.value) {
                return Err(format!("{track} mixes kinds of values"));
            }
        }
        Ok(())
    }
}

impl MethodCall {
    fn validate(&self, length: f32) -> Result<(), String> {
        validate_path(&self.node)?;
        let node = node_name(&self.node);
        if self.method.trim().is_empty() {
            return Err(format!("a call to {node} names no method"));
        }
        if !within_length(self.time, length) {
            return Err(format!(
                "the call to `{}` of {node} is at {}, outside the animation's {length} seconds",
                self.method, self.time
            ));
        }
        Ok(())
    }
}

/// How messages name the node at `path`.
fn node_name(path: &str) -> String {
    if path.is_empty() {
        "the node animated".to_owned()
    } else {
        format!("\"{path}\"")
    }
}

/// Checks that `path` is empty or names each node on the way.
fn validate_path(path: &str) -> Result<(), String> {
    if !path.is_empty() && path.split(PATH_SEPARATOR).any(str::is_empty) {
        return Err(format!(
            "the path \"{path}\" has an empty name: nodes are separated by one `{PATH_SEPARATOR}`"
        ));
    }
    Ok(())
}

fn within_length(time: f32, length: f32) -> bool {
    time.is_finite() && (0.0..=length).contains(&time)
}

/// Whether `first` and `other` are values of the same kind; whole numbers
/// and decimals are both numbers.
fn same_kind(first: &FieldValue, other: &FieldValue) -> bool {
    let kind = |value: &FieldValue| match value {
        FieldValue::Bool(_) => 0,
        FieldValue::Int(_) | FieldValue::Float(_) => 1,
        FieldValue::Text(_) => 2,
        FieldValue::Color(_) => 3,
    };
    kind(first) == kind(other)
}

fn invalid(reason: &str) -> EngineError {
    EngineError::Asset(reason.to_owned())
}

/// A whole `.anim` file.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AnimationsFile {
    format: u32,
    #[serde(rename = "animation", default)]
    animations: Vec<AnimationData>,
}

/// One `[[animation]]` table.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AnimationData {
    name: String,
    length: f32,
    #[serde(default)]
    looping: bool,
    #[serde(rename = "track", default, skip_serializing_if = "Vec::is_empty")]
    tracks: Vec<TrackData>,
    #[serde(rename = "call", default, skip_serializing_if = "Vec::is_empty")]
    calls: Vec<CallData>,
}

/// One `[[animation.track]]` table: its keys as `[time, value]` pairs.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TrackData {
    #[serde(default)]
    node: String,
    property: String,
    #[serde(default)]
    interpolation: Interpolation,
    keys: Vec<(f32, FieldValue)>,
}

/// One `[[animation.call]]` table.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CallData {
    #[serde(default)]
    node: String,
    time: f32,
    method: String,
}

impl From<AnimationData> for TrackAnimation {
    fn from(data: AnimationData) -> Self {
        Self {
            name: data.name,
            length: data.length,
            looping: data.looping,
            tracks: data
                .tracks
                .into_iter()
                .map(|track| PropertyTrack {
                    node: track.node,
                    property: track.property,
                    interpolation: track.interpolation,
                    keys: track
                        .keys
                        .into_iter()
                        .map(|(time, value)| Key { time, value })
                        .collect(),
                })
                .collect(),
            calls: data
                .calls
                .into_iter()
                .map(|call| MethodCall {
                    node: call.node,
                    time: call.time,
                    method: call.method,
                })
                .collect(),
        }
    }
}

impl From<&TrackAnimation> for AnimationData {
    fn from(animation: &TrackAnimation) -> Self {
        Self {
            name: animation.name.clone(),
            length: animation.length,
            looping: animation.looping,
            tracks: animation
                .tracks
                .iter()
                .map(|track| TrackData {
                    node: track.node.clone(),
                    property: track.property.clone(),
                    interpolation: track.interpolation,
                    keys: track
                        .keys
                        .iter()
                        .map(|key| (key.time, key.value.clone()))
                        .collect(),
                })
                .collect(),
            calls: animation
                .calls
                .iter()
                .map(|call| CallData {
                    node: call.node.clone(),
                    time: call.time,
                    method: call.method.clone(),
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KNIGHT: &str = r#"
        format = 1

        [[animation]]
        name = "attack"
        length = 0.6

        [[animation.track]]
        node = "Arm"
        property = "rotation"
        interpolation = "ease_out"
        keys = [[0.0, 0.0], [0.3, 1.2], [0.6, 0]]

        [[animation.track]]
        property = "color"
        interpolation = "step"
        keys = [[0.0, [1.0, 1.0, 1.0, 1.0]], [0.3, [1.0, 0.2, 0.2, 1.0]]]

        [[animation.call]]
        time = 0.3
        method = "hit"

        [[animation]]
        name = "idle"
        length = 2.0
        looping = true

        [[animation.track]]
        property = "visible"
        keys = [[0.0, true], [1.0, false]]
    "#;

    fn error_of(text: &str) -> String {
        TrackAnimations::from_text(&format!("format = 1\n{text}"))
            .unwrap_err()
            .to_string()
    }

    fn attack(tracks: &str) -> String {
        format!("[[animation]]\nname = \"attack\"\nlength = 1.0\n{tracks}")
    }

    #[test]
    fn files_are_read_with_their_tracks_and_calls() {
        let file = TrackAnimations::from_text(KNIGHT).unwrap();

        let attack = file.animation("attack").unwrap();
        assert!(!attack.looping);
        let arm = &attack.tracks[0];
        assert_eq!(
            (arm.node.as_str(), arm.property.as_str()),
            ("Arm", "rotation")
        );
        assert_eq!(arm.interpolation, Interpolation::EaseOut);
        assert_eq!(arm.keys[1].value, FieldValue::Float(1.2));
        assert_eq!(arm.keys[2].value, FieldValue::Int(0));
        assert_eq!(attack.tracks[1].node, "");
        assert_eq!(
            attack.calls,
            [MethodCall {
                node: String::new(),
                time: 0.3,
                method: "hit".to_owned(),
            }]
        );
        let idle = file.animation("idle").unwrap();
        assert!(idle.looping);
        assert_eq!(idle.tracks[0].interpolation, Interpolation::Linear);
        assert_eq!(idle.tracks[0].keys[1].value, FieldValue::Bool(false));
    }

    #[test]
    fn files_are_written_and_read_back_the_same() {
        let file = TrackAnimations::from_text(KNIGHT).unwrap();

        let written = file.to_text().unwrap();

        assert_eq!(TrackAnimations::from_text(&written).unwrap(), file);
    }

    #[test]
    fn animations_need_a_name_of_their_own_and_a_length() {
        assert!(error_of("[[animation]]\nname = \" \"\nlength = 1.0").contains("no name"));
        assert!(error_of("[[animation]]\nname = \"a\"\nlength = 0.0").contains("above zero"));
        let twice = "[[animation]]\nname = \"a\"\nlength = 1.0\n".repeat(2);
        assert!(error_of(&twice).contains("two animations"));
        assert!(
            TrackAnimations::from_text("format = 2")
                .unwrap_err()
                .to_string()
                .contains("format 2")
        );
    }

    #[test]
    fn keys_must_be_in_order_within_the_length_and_of_one_kind() {
        let track = |keys: &str| {
            error_of(&attack(&format!(
                "[[animation.track]]\nproperty = \"x\"\nkeys = {keys}"
            )))
        };

        assert!(track("[]").contains("no keys"));
        assert!(track("[[0.0, 1.0], [1.5, 2.0]]").contains("outside"));
        assert!(track("[[0.5, 1.0], [0.5, 2.0]]").contains("out of order"));
        assert!(track("[[0.5, 1.0], [0.2, 2.0]]").contains("out of order"));
        assert!(track("[[0.0, 1.0], [0.5, true]]").contains("mixes"));
    }

    #[test]
    fn tracks_and_calls_need_valid_paths_and_names() {
        let error = error_of(&attack(
            "[[animation.track]]\nnode = \"Arm//Hand\"\nproperty = \"x\"\nkeys = [[0.0, 1.0]]",
        ));
        assert!(error.contains("empty name"), "{error}");
        let error = error_of(&attack(
            "[[animation.track]]\nproperty = \"\"\nkeys = [[0.0, 1.0]]",
        ));
        assert!(error.contains("no property"), "{error}");
        let twice = "[[animation.track]]\nnode = \"Arm\"\nproperty = \"x\"\nkeys = [[0.0, 1.0]]\n"
            .repeat(2);
        let error = error_of(&attack(&twice));
        assert!(
            error.contains("two tracks change `x` of \"Arm\""),
            "{error}"
        );
        let error = error_of(&attack("[[animation.call]]\ntime = 0.5\nmethod = \"\""));
        assert!(error.contains("no method"), "{error}");
        let error = error_of(&attack("[[animation.call]]\ntime = 2.0\nmethod = \"hit\""));
        assert!(error.contains("outside"), "{error}");
    }
}
