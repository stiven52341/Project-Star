//! Changes to the animations of an `.anim` file, as the timeline makes
//! them: keys and calls added, moved, copied and deleted, and animations
//! created, renamed, duplicated and deleted. Each change leaves the
//! animations valid (see `TrackAnimations::validate`).

use star_engine::star_scene::{
    FieldValue, Interpolation, Key, MethodCall, PropertyTrack, TrackAnimation, TrackAnimations,
};

/// How finely times are kept: keys a thousandth of a second apart are the
/// same moment.
const STEPS_PER_SECOND: f32 = 1000.0;
/// The time between two moments, in seconds.
const TIME_STEP: f32 = 1.0 / STEPS_PER_SECOND;

/// The shortest an animation can last, in seconds.
pub(super) const MIN_LENGTH: f32 = 0.01;

/// The length of a new animation, in seconds.
const NEW_LENGTH: f32 = 1.0;

/// What a new animation is called, before a number makes it unique.
const NEW_NAME: &str = "new_animation";

/// What a copy's name ends with.
const COPY_SUFFIX: &str = "_copy";

/// A key or a call of an animation, as the timeline selects them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Mark {
    /// The key `key` of the track `track`.
    Key { track: usize, key: usize },
    /// The call `call`.
    Call(usize),
}

/// Keys and calls copied, with their times from the first one's.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct Copied {
    keys: Vec<(String, String, f32, FieldValue)>,
    calls: Vec<(String, f32, String)>,
}

impl Copied {
    /// Whether nothing was copied.
    pub(super) const fn is_empty(&self) -> bool {
        self.keys.is_empty() && self.calls.is_empty()
    }
}

/// `time` kept to [`TIME_STEP`].
fn rounded(time: f32) -> f32 {
    (time * STEPS_PER_SECOND).round() / STEPS_PER_SECOND
}

fn same_time(first: f32, second: f32) -> bool {
    (first - second).abs() < TIME_STEP / 2.0
}

/// Whether `track` has a key at `time`.
pub(super) fn has_key_at(track: &PropertyTrack, time: f32) -> bool {
    let time = rounded(time);
    track.keys.iter().any(|key| same_time(key.time, time))
}

/// The time of `mark` in `animation`, if it is there.
pub(super) fn mark_time(animation: &TrackAnimation, mark: Mark) -> Option<f32> {
    match mark {
        Mark::Key { track, key } => Some(animation.tracks.get(track)?.keys.get(key)?.time),
        Mark::Call(call) => Some(animation.calls.get(call)?.time),
    }
}

/// Whether two values can be keys of one track: numbers with numbers,
/// colors with colors, and so on.
fn same_kind(first: &FieldValue, second: &FieldValue) -> bool {
    std::mem::discriminant(first) == std::mem::discriminant(second)
        || matches!(
            (first, second),
            (
                FieldValue::Int(_) | FieldValue::Float(_),
                FieldValue::Int(_) | FieldValue::Float(_)
            )
        )
}

/// Gives `property` of the node at `node` the key `value` at `time`: in its
/// track, made if there is none, replacing a key at that same time.
/// Returns the key's mark, or `None` if the track holds values of another
/// kind.
pub(super) fn set_key(
    animation: &mut TrackAnimation,
    node: &str,
    property: &str,
    time: f32,
    value: FieldValue,
) -> Option<Mark> {
    let time = rounded(time.clamp(0.0, animation.length));
    let found = animation
        .tracks
        .iter()
        .position(|track| track.node == node && track.property == property);
    let track = found.unwrap_or_else(|| {
        animation.tracks.push(PropertyTrack {
            node: node.to_owned(),
            property: property.to_owned(),
            interpolation: Interpolation::default(),
            keys: Vec::new(),
        });
        animation.tracks.len() - 1
    });
    let keys = &mut animation.tracks[track].keys;
    if keys
        .first()
        .is_some_and(|first| !same_kind(&first.value, &value))
    {
        return None;
    }
    let key = keys.partition_point(|key| key.time < time - TIME_STEP / 2.0);
    if keys.get(key).is_some_and(|old| same_time(old.time, time)) {
        keys[key].value = value;
    } else {
        keys.insert(key, Key { time, value });
    }
    Some(Mark::Key { track, key })
}

/// Adds a call to `method` of the node at `node` at `time`. Returns its
/// mark.
pub(super) fn add_call(
    animation: &mut TrackAnimation,
    node: &str,
    time: f32,
    method: &str,
) -> Mark {
    animation.calls.push(MethodCall {
        node: node.to_owned(),
        time: rounded(time.clamp(0.0, animation.length)),
        method: method.to_owned(),
    });
    Mark::Call(animation.calls.len() - 1)
}

/// Deletes the keys and calls of `marks`; a track left without keys goes
/// too.
pub(super) fn delete_marks(animation: &mut TrackAnimation, marks: &[Mark]) {
    for (track_index, track) in animation.tracks.iter_mut().enumerate() {
        let mut key = 0;
        track.keys.retain(|_| {
            key += 1;
            !marks.contains(&Mark::Key {
                track: track_index,
                key: key - 1,
            })
        });
    }
    animation.tracks.retain(|track| !track.keys.is_empty());
    let mut call = 0;
    animation.calls.retain(|_| {
        call += 1;
        !marks.contains(&Mark::Call(call - 1))
    });
}

/// Moves the keys and calls of `marks` by `shift` seconds, within the
/// animation. A key that lands on another key of its track takes its
/// place. Returns the marks of what moved, where it is now.
pub(super) fn move_marks(animation: &mut TrackAnimation, marks: &[Mark], shift: f32) -> Vec<Mark> {
    let length = animation.length;
    let moved_time = |time: f32| rounded((time + shift).clamp(0.0, length));
    let mut moved = Vec::new();
    for (track_index, track) in animation.tracks.iter_mut().enumerate() {
        let mut keys: Vec<(bool, Key)> = std::mem::take(&mut track.keys)
            .into_iter()
            .enumerate()
            .map(|(key, mut value)| {
                let chosen = marks.contains(&Mark::Key {
                    track: track_index,
                    key,
                });
                if chosen {
                    value.time = moved_time(value.time);
                }
                (chosen, value)
            })
            .collect();
        // Moved keys first among those at one time, so they are the ones
        // kept.
        keys.sort_by(|(first_moved, first), (second_moved, second)| {
            first
                .time
                .total_cmp(&second.time)
                .then(second_moved.cmp(first_moved))
        });
        keys.dedup_by(|later, kept| same_time(later.1.time, kept.1.time));
        for (key, (chosen, value)) in keys.into_iter().enumerate() {
            if chosen {
                moved.push(Mark::Key {
                    track: track_index,
                    key,
                });
            }
            track.keys.push(value);
        }
    }
    for (index, call) in animation.calls.iter_mut().enumerate() {
        if marks.contains(&Mark::Call(index)) {
            call.time = moved_time(call.time);
            moved.push(Mark::Call(index));
        }
    }
    moved
}

/// Copies the keys and calls of `marks`.
pub(super) fn copy_marks(animation: &TrackAnimation, marks: &[Mark]) -> Copied {
    let start = marks
        .iter()
        .filter_map(|&mark| mark_time(animation, mark))
        .reduce(f32::min)
        .unwrap_or_default();
    let mut copied = Copied::default();
    for &mark in marks {
        match mark {
            Mark::Key { track, key } => {
                let Some(track) = animation.tracks.get(track) else {
                    continue;
                };
                if let Some(key) = track.keys.get(key) {
                    copied.keys.push((
                        track.node.clone(),
                        track.property.clone(),
                        key.time - start,
                        key.value.clone(),
                    ));
                }
            }
            Mark::Call(call) => {
                if let Some(call) = animation.calls.get(call) {
                    copied
                        .calls
                        .push((call.node.clone(), call.time - start, call.method.clone()));
                }
            }
        }
    }
    // In the order of their times, so pasting each one leaves the places
    // of those pasted before it in their track as they were.
    copied
        .keys
        .sort_by(|first, second| first.2.total_cmp(&second.2));
    copied
}

/// Puts what was copied at `time`, leaving out what would fall past the
/// end. Returns the marks of what was put.
pub(super) fn paste(animation: &mut TrackAnimation, copied: &Copied, time: f32) -> Vec<Mark> {
    let length = animation.length;
    let fits = |offset: f32| time + offset <= length + TIME_STEP / 2.0;
    let mut pasted = Vec::new();
    for (node, property, offset, value) in &copied.keys {
        if fits(*offset) {
            pasted.extend(set_key(
                animation,
                node,
                property,
                time + offset,
                value.clone(),
            ));
        }
    }
    for (node, offset, method) in &copied.calls {
        if fits(*offset) {
            pasted.push(add_call(animation, node, time + offset, method));
        }
    }
    pasted
}

/// The time of the last key or call: an animation cannot be shorter.
pub(super) fn last_time(animation: &TrackAnimation) -> f32 {
    animation
        .tracks
        .iter()
        .flat_map(|track| track.keys.iter().map(|key| key.time))
        .chain(animation.calls.iter().map(|call| call.time))
        .fold(0.0, f32::max)
}

/// A name of `base` no animation of `file` has: `base`, or `base` and a
/// number.
fn free_name(file: &TrackAnimations, base: &str) -> String {
    let taken = |name: &str| file.animation(name).is_some();
    if !taken(base) {
        return base.to_owned();
    }
    // One more number than there are animations is always free.
    (2..=file.animations.len() + 1)
        .map(|number| format!("{base}_{number}"))
        .find(|name| !taken(name))
        .unwrap_or_else(|| base.to_owned())
}

/// Adds an empty animation with a free name. Returns its name.
pub(super) fn add_animation(file: &mut TrackAnimations) -> String {
    let name = free_name(file, NEW_NAME);
    file.animations.push(TrackAnimation {
        name: name.clone(),
        length: NEW_LENGTH,
        looping: false,
        tracks: Vec::new(),
        calls: Vec::new(),
    });
    name
}

/// Adds a copy of the animation `name` with a free name. Returns the
/// copy's name.
pub(super) fn duplicate_animation(file: &mut TrackAnimations, name: &str) -> Option<String> {
    let mut copy = file.animation(name)?.clone();
    copy.name = free_name(file, &format!("{name}{COPY_SUFFIX}"));
    let copy_name = copy.name.clone();
    file.animations.push(copy);
    Some(copy_name)
}

/// Calls the animation `from` `to`. Returns whether it could: names are
/// not blank and each animation has its own.
pub(super) fn rename_animation(file: &mut TrackAnimations, from: &str, to: &str) -> bool {
    let to = to.trim();
    if to.is_empty() || file.animation(to).is_some() {
        return false;
    }
    let Some(animation) = file
        .animations
        .iter_mut()
        .find(|animation| animation.name == from)
    else {
        return false;
    };
    to.clone_into(&mut animation.name);
    true
}

/// Deletes the animation `name`.
pub(super) fn delete_animation(file: &mut TrackAnimations, name: &str) {
    file.animations.retain(|animation| animation.name != name);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attack() -> TrackAnimation {
        TrackAnimation {
            name: "attack".to_owned(),
            length: 1.0,
            looping: false,
            tracks: Vec::new(),
            calls: Vec::new(),
        }
    }

    fn times(animation: &TrackAnimation, track: usize) -> Vec<f32> {
        animation.tracks[track]
            .keys
            .iter()
            .map(|key| key.time)
            .collect()
    }

    fn file_of(animation: TrackAnimation) -> TrackAnimations {
        TrackAnimations {
            animations: vec![animation],
        }
    }

    #[test]
    fn keys_make_their_track_in_order_and_replace_one_at_the_same_time() {
        let mut animation = attack();

        set_key(
            &mut animation,
            "Arm",
            "rotation",
            0.5,
            FieldValue::Float(1.0),
        );
        let first = set_key(&mut animation, "Arm", "rotation", 0.0, FieldValue::Int(0));
        let again = set_key(
            &mut animation,
            "Arm",
            "rotation",
            0.5004,
            FieldValue::Float(2.0),
        );

        assert_eq!(first, Some(Mark::Key { track: 0, key: 0 }));
        assert_eq!(again, Some(Mark::Key { track: 0, key: 1 }));
        assert_eq!(times(&animation, 0), [0.0, 0.5]);
        assert_eq!(animation.tracks[0].keys[1].value, FieldValue::Float(2.0));
        let wrong = set_key(
            &mut animation,
            "Arm",
            "rotation",
            0.2,
            FieldValue::Bool(true),
        );
        assert_eq!(wrong, None);
        file_of(animation).validate().unwrap();
    }

    #[test]
    fn deleting_a_tracks_last_key_deletes_the_track() {
        let mut animation = attack();
        set_key(&mut animation, "", "x", 0.0, FieldValue::Float(0.0));
        set_key(&mut animation, "", "y", 0.0, FieldValue::Float(0.0));
        set_key(&mut animation, "", "y", 1.0, FieldValue::Float(1.0));
        add_call(&mut animation, "", 0.5, "hit");

        delete_marks(
            &mut animation,
            &[
                Mark::Key { track: 0, key: 0 },
                Mark::Key { track: 1, key: 1 },
                Mark::Call(0),
            ],
        );

        assert_eq!(animation.tracks.len(), 1);
        assert_eq!(animation.tracks[0].property, "y");
        assert_eq!(times(&animation, 0), [0.0]);
        assert_eq!(animation.calls, []);
    }

    #[test]
    fn moved_keys_stay_within_the_animation_and_take_the_place_of_others() {
        let mut animation = attack();
        for time in [0.0, 0.25, 0.5] {
            set_key(
                &mut animation,
                "",
                "x",
                time,
                FieldValue::Float(f64::from(time)),
            );
        }
        add_call(&mut animation, "", 0.75, "hit");

        let moved = move_marks(
            &mut animation,
            &[Mark::Key { track: 0, key: 0 }, Mark::Call(0)],
            0.5,
        );

        assert_eq!(times(&animation, 0), [0.25, 0.5]);
        assert_eq!(animation.tracks[0].keys[1].value, FieldValue::Float(0.0));
        assert_eq!(animation.calls[0].time, 1.0);
        assert_eq!(moved, [Mark::Key { track: 0, key: 1 }, Mark::Call(0)]);
        file_of(animation).validate().unwrap();
    }

    #[test]
    fn copied_keys_are_pasted_from_a_time_and_what_falls_past_the_end_is_left_out() {
        let mut animation = attack();
        set_key(&mut animation, "", "x", 0.2, FieldValue::Float(1.0));
        set_key(&mut animation, "", "x", 0.4, FieldValue::Float(2.0));
        add_call(&mut animation, "Arm", 0.3, "swing");
        let copied = copy_marks(
            &animation,
            &[
                Mark::Key { track: 0, key: 0 },
                Mark::Key { track: 0, key: 1 },
                Mark::Call(0),
            ],
        );

        let pasted = paste(&mut animation, &copied, 0.7);

        assert_eq!(times(&animation, 0), [0.2, 0.4, 0.7, 0.9]);
        assert_eq!(animation.calls[1].time, 0.8);
        assert_eq!(animation.calls[1].node, "Arm");
        assert_eq!(pasted.len(), 3);
        let late = paste(&mut animation, &copied, 0.9);
        assert_eq!(late, [Mark::Key { track: 0, key: 3 }, Mark::Call(2)]);
        assert!(!copied.is_empty());
    }

    #[test]
    fn animations_get_free_names() {
        let mut file = TrackAnimations::default();

        let first = add_animation(&mut file);
        let second = add_animation(&mut file);
        let copy = duplicate_animation(&mut file, &first).unwrap();

        assert_eq!(
            [first.as_str(), second.as_str()],
            [NEW_NAME, "new_animation_2"]
        );
        assert_eq!(copy, "new_animation_copy");
        assert!(!rename_animation(&mut file, &copy, &second));
        assert!(!rename_animation(&mut file, &copy, " "));
        assert!(rename_animation(&mut file, &copy, " jump "));
        delete_animation(&mut file, &first);
        let names: Vec<&str> = file
            .animations
            .iter()
            .map(|animation| animation.name.as_str())
            .collect();
        assert_eq!(names, ["new_animation_2", "jump"]);
        file.validate().unwrap();
    }

    #[test]
    fn animations_last_at_least_until_their_last_mark() {
        let mut animation = attack();
        set_key(&mut animation, "", "x", 0.4, FieldValue::Float(1.0));
        add_call(&mut animation, "", 0.6, "hit");

        assert!((last_time(&animation) - 0.6).abs() < 1e-6);
    }
}
