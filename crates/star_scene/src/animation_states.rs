//! `.states` files: state machines that choose which animations of an
//! `.anim` file an `Animator` plays.
//!
//! Scripts set the machine's values (numbers, flags and triggers); each
//! frame a transition whose conditions hold takes the machine from its
//! state to another, fading from one's animation to the other's. A state
//! plays one animation, or blends several by a number (walk and run by
//! speed).

use serde::{Deserialize, Serialize};
use star_core::{EngineError, EngineResult};

/// The version of the layout this engine writes and reads.
const FORMAT_VERSION: u32 = 1;

/// The extension of the files, with its dot.
pub const STATES_EXTENSION: &str = ".states";

/// The state machine of a `.states` file.
#[derive(Debug, Clone, PartialEq)]
pub struct AnimationStates {
    /// The values scripts set, each with a different name.
    pub values: Vec<StateValue>,
    /// The state the machine starts in.
    pub start: String,
    /// The states, each with a different name; never empty.
    pub states: Vec<AnimationState>,
    /// The ways from a state to another, tried in this order.
    pub transitions: Vec<StateTransition>,
}

/// A value of the machine, set by scripts and read by conditions.
#[derive(Debug, Clone, PartialEq)]
pub struct StateValue {
    /// What scripts and conditions name it by (`"speed"`).
    pub name: String,
    /// What it holds.
    pub kind: ValueKind,
}

/// What a [`StateValue`] holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueKind {
    /// A decimal number, zero at the start.
    Number,
    /// `true` or `false`, `false` at the start.
    Flag,
    /// Set by a script for one frame, to start a transition once.
    Trigger,
}

/// A state of the machine.
#[derive(Debug, Clone, PartialEq)]
pub struct AnimationState {
    /// What transitions and scripts name it by (`"idle"`).
    pub name: String,
    /// What it plays.
    pub motion: Motion,
    /// Where the editor draws it on the graph.
    pub position: [f32; 2],
}

/// What an [`AnimationState`] plays.
#[derive(Debug, Clone, PartialEq)]
pub enum Motion {
    /// One animation of the animator's `.anim` file, by name.
    Animation(String),
    /// Animations blended by where a number value is between their
    /// points.
    Blend {
        /// The number value that chooses the blend.
        value: String,
        /// The animations and where each plays alone, in order; never
        /// empty.
        points: Vec<BlendPoint>,
    },
}

/// An animation of a [`Motion::Blend`] and the value where it plays alone.
#[derive(Debug, Clone, PartialEq)]
pub struct BlendPoint {
    /// The animation, by name.
    pub animation: String,
    /// The value where it plays alone.
    pub at: f32,
}

/// A way from a state to another.
#[derive(Debug, Clone, PartialEq)]
pub struct StateTransition {
    /// The state it leaves, or `None` for any state but `to`.
    pub from: Option<String>,
    /// The state it enters.
    pub to: String,
    /// What must hold to take it; all of them.
    pub conditions: Vec<Condition>,
    /// Seconds fading from the state's animation to the next one's; zero
    /// to cut.
    pub fade: f32,
    /// Whether it waits for the state's animation to reach its end (a
    /// looping one, the end of a round).
    pub at_end: bool,
}

/// Something a [`StateTransition`] needs of a value.
#[derive(Debug, Clone, PartialEq)]
pub struct Condition {
    /// The value, by name.
    pub value: String,
    /// What it must be.
    pub test: Test,
}

/// What a [`Condition`] tests.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Test {
    /// A number above this one.
    Above(f32),
    /// A number below this one.
    Below(f32),
    /// A flag with this value.
    Is(bool),
    /// A trigger set in this frame.
    Triggered,
}

impl AnimationStates {
    /// Reads the text of a `.states` file.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Asset`] if it is not valid TOML of this
    /// layout, or its values do not make a valid machine (see
    /// [`AnimationStates::validate`]).
    pub fn from_text(text: &str) -> EngineResult<Self> {
        let file: StatesFile = toml::from_str(text)
            .map_err(|error| invalid(&format!("invalid states file: {error}")))?;
        if file.format != FORMAT_VERSION {
            return Err(invalid(&format!(
                "the states file has format {}, but this engine reads format {FORMAT_VERSION}",
                file.format
            )));
        }
        let states = Self::try_from(file)?;
        states.validate()?;
        Ok(states)
    }

    /// Writes the text of a `.states` file.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Asset`] if its values do not make a valid
    /// machine (see [`AnimationStates::validate`]).
    pub fn to_text(&self) -> EngineResult<String> {
        self.validate()?;
        toml::to_string(&StatesFile::from(self))
            .map_err(|error| invalid(&format!("cannot write the states file: {error}")))
    }

    /// The state called `name`.
    #[must_use]
    pub fn state(&self, name: &str) -> Option<&AnimationState> {
        self.states.iter().find(|state| state.name == name)
    }

    /// The value called `name`.
    #[must_use]
    pub fn value(&self, name: &str) -> Option<&StateValue> {
        self.values.iter().find(|value| value.name == name)
    }

    /// Checks that values and states have names of their own, that the
    /// machine starts in one of its states, that blends take a number and
    /// points in order, and that transitions join two different states
    /// with a fade of zero or more and conditions that fit their values.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Asset`] naming the first value that is not.
    pub fn validate(&self) -> EngineResult<()> {
        self.validate_values()?;
        self.validate_states()?;
        if self.state(&self.start).is_none() {
            return Err(invalid(&format!(
                "the machine starts in \"{}\", which is not one of its states",
                self.start
            )));
        }
        self.transitions
            .iter()
            .try_for_each(|transition| self.validate_transition(transition))
    }

    fn validate_values(&self) -> EngineResult<()> {
        for (index, value) in self.values.iter().enumerate() {
            if value.name.trim().is_empty() {
                return Err(invalid("a value has no name"));
            }
            if self.values[..index]
                .iter()
                .any(|other| other.name == value.name)
            {
                return Err(invalid(&format!(
                    "two values are called \"{}\"",
                    value.name
                )));
            }
        }
        Ok(())
    }

    fn validate_states(&self) -> EngineResult<()> {
        if self.states.is_empty() {
            return Err(invalid("the machine has no states"));
        }
        for (index, state) in self.states.iter().enumerate() {
            if state.name.trim().is_empty() {
                return Err(invalid("a state has no name"));
            }
            if self.states[..index]
                .iter()
                .any(|other| other.name == state.name)
            {
                return Err(invalid(&format!(
                    "two states are called \"{}\"",
                    state.name
                )));
            }
            self.validate_motion(&state.motion)
                .map_err(|reason| invalid(&format!("the state \"{}\": {reason}", state.name)))?;
        }
        Ok(())
    }

    fn validate_motion(&self, motion: &Motion) -> Result<(), String> {
        match motion {
            Motion::Animation(animation) => validate_animation_name(animation),
            Motion::Blend { value, points } => {
                self.expect_kind(value, ValueKind::Number)?;
                if points.is_empty() {
                    return Err("its blend has no points".to_owned());
                }
                let mut previous: Option<f32> = None;
                for point in points {
                    validate_animation_name(&point.animation)?;
                    if !point.at.is_finite() || previous.is_some_and(|before| point.at <= before) {
                        return Err(
                            "its blend points are out of order or at the same value".to_owned()
                        );
                    }
                    previous = Some(point.at);
                }
                Ok(())
            }
        }
    }

    fn validate_transition(&self, transition: &StateTransition) -> EngineResult<()> {
        let from = transition.from.as_deref();
        let name = format!(
            "the transition from {} to \"{}\"",
            from.map_or_else(|| "any state".to_owned(), |from| format!("\"{from}\"")),
            transition.to
        );
        let within = |reason: String| invalid(&format!("{name}: {reason}"));
        for state in from.into_iter().chain([transition.to.as_str()]) {
            if self.state(state).is_none() {
                return Err(within(format!("there is no state \"{state}\"")));
            }
        }
        if from == Some(transition.to.as_str()) {
            return Err(within("it leaves and enters the same state".to_owned()));
        }
        if !(transition.fade.is_finite() && transition.fade >= 0.0) {
            return Err(within("its fade must be zero or more seconds".to_owned()));
        }
        transition
            .conditions
            .iter()
            .try_for_each(|condition| self.validate_condition(condition))
            .map_err(within)
    }

    fn validate_condition(&self, condition: &Condition) -> Result<(), String> {
        let kind = match condition.test {
            Test::Above(number) | Test::Below(number) if !number.is_finite() => {
                return Err(format!(
                    "the condition on \"{}\" needs a finite number",
                    condition.value
                ));
            }
            Test::Above(_) | Test::Below(_) => ValueKind::Number,
            Test::Is(_) => ValueKind::Flag,
            Test::Triggered => ValueKind::Trigger,
        };
        self.expect_kind(&condition.value, kind)
    }

    /// Checks that the value called `name` exists and holds `kind`.
    fn expect_kind(&self, name: &str, kind: ValueKind) -> Result<(), String> {
        match self.value(name) {
            None => Err(format!("there is no value \"{name}\"")),
            Some(value) if value.kind != kind => Err(format!(
                "\"{name}\" is a {}, but a {} is needed",
                kind_name(value.kind),
                kind_name(kind)
            )),
            Some(_) => Ok(()),
        }
    }
}

fn validate_animation_name(animation: &str) -> Result<(), String> {
    if animation.trim().is_empty() {
        return Err("it names no animation".to_owned());
    }
    Ok(())
}

fn kind_name(kind: ValueKind) -> &'static str {
    match kind {
        ValueKind::Number => "number",
        ValueKind::Flag => "flag",
        ValueKind::Trigger => "trigger",
    }
}

fn invalid(reason: &str) -> EngineError {
    EngineError::Asset(reason.to_owned())
}

/// A whole `.states` file.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StatesFile {
    format: u32,
    start: String,
    #[serde(rename = "value", default, skip_serializing_if = "Vec::is_empty")]
    values: Vec<ValueData>,
    #[serde(rename = "state", default)]
    states: Vec<StateData>,
    #[serde(rename = "transition", default, skip_serializing_if = "Vec::is_empty")]
    transitions: Vec<TransitionData>,
}

/// One `[[value]]` table.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ValueData {
    name: String,
    kind: ValueKind,
}

/// One `[[state]]` table: an `animation`, or a `blend` value with its
/// `points`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StateData {
    name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    animation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    blend: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    points: Vec<PointData>,
    #[serde(default)]
    position: [f32; 2],
}

/// One point of a state's `points`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PointData {
    animation: String,
    at: f32,
}

/// One `[[transition]]` table; without `from`, it leaves any state.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TransitionData {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    from: Option<String>,
    to: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    conditions: Vec<ConditionData>,
    #[serde(default)]
    fade: f32,
    #[serde(default)]
    at_end: bool,
}

/// One condition of a transition: `above`, `below` or `is`, or none of
/// them for a trigger.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConditionData {
    value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    above: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    below: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    is: Option<bool>,
}

impl TryFrom<StatesFile> for AnimationStates {
    type Error = EngineError;

    fn try_from(file: StatesFile) -> EngineResult<Self> {
        Ok(Self {
            values: file
                .values
                .into_iter()
                .map(|value| StateValue {
                    name: value.name,
                    kind: value.kind,
                })
                .collect(),
            start: file.start,
            states: file
                .states
                .into_iter()
                .map(AnimationState::try_from)
                .collect::<EngineResult<_>>()?,
            transitions: file
                .transitions
                .into_iter()
                .map(StateTransition::try_from)
                .collect::<EngineResult<_>>()?,
        })
    }
}

impl TryFrom<StateData> for AnimationState {
    type Error = EngineError;

    fn try_from(data: StateData) -> EngineResult<Self> {
        let motion = match (data.animation, data.blend) {
            (Some(animation), None) if data.points.is_empty() => Motion::Animation(animation),
            (None, Some(value)) => Motion::Blend {
                value,
                points: data
                    .points
                    .into_iter()
                    .map(|point| BlendPoint {
                        animation: point.animation,
                        at: point.at,
                    })
                    .collect(),
            },
            _ => {
                return Err(invalid(&format!(
                    "the state \"{}\" needs an `animation`, or a `blend` value with `points`",
                    data.name
                )));
            }
        };
        Ok(Self {
            name: data.name,
            motion,
            position: data.position,
        })
    }
}

impl TryFrom<TransitionData> for StateTransition {
    type Error = EngineError;

    fn try_from(data: TransitionData) -> EngineResult<Self> {
        Ok(Self {
            from: data.from,
            to: data.to,
            conditions: data
                .conditions
                .into_iter()
                .map(Condition::try_from)
                .collect::<EngineResult<_>>()?,
            fade: data.fade,
            at_end: data.at_end,
        })
    }
}

impl TryFrom<ConditionData> for Condition {
    type Error = EngineError;

    fn try_from(data: ConditionData) -> EngineResult<Self> {
        let test = match (data.above, data.below, data.is) {
            (None, None, None) => Test::Triggered,
            (Some(above), None, None) => Test::Above(above),
            (None, Some(below), None) => Test::Below(below),
            (None, None, Some(is)) => Test::Is(is),
            _ => {
                return Err(invalid(&format!(
                    "the condition on \"{}\" takes one of `above`, `below` or `is`",
                    data.value
                )));
            }
        };
        Ok(Self {
            value: data.value,
            test,
        })
    }
}

impl From<&AnimationStates> for StatesFile {
    fn from(states: &AnimationStates) -> Self {
        Self {
            format: FORMAT_VERSION,
            start: states.start.clone(),
            values: states
                .values
                .iter()
                .map(|value| ValueData {
                    name: value.name.clone(),
                    kind: value.kind,
                })
                .collect(),
            states: states.states.iter().map(StateData::from).collect(),
            transitions: states
                .transitions
                .iter()
                .map(TransitionData::from)
                .collect(),
        }
    }
}

impl From<&AnimationState> for StateData {
    fn from(state: &AnimationState) -> Self {
        let (animation, blend, points) = match &state.motion {
            Motion::Animation(animation) => (Some(animation.clone()), None, Vec::new()),
            Motion::Blend { value, points } => (
                None,
                Some(value.clone()),
                points
                    .iter()
                    .map(|point| PointData {
                        animation: point.animation.clone(),
                        at: point.at,
                    })
                    .collect(),
            ),
        };
        Self {
            name: state.name.clone(),
            animation,
            blend,
            points,
            position: state.position,
        }
    }
}

impl From<&StateTransition> for TransitionData {
    fn from(transition: &StateTransition) -> Self {
        Self {
            from: transition.from.clone(),
            to: transition.to.clone(),
            conditions: transition
                .conditions
                .iter()
                .map(ConditionData::from)
                .collect(),
            fade: transition.fade,
            at_end: transition.at_end,
        }
    }
}

impl From<&Condition> for ConditionData {
    fn from(condition: &Condition) -> Self {
        let mut data = Self {
            value: condition.value.clone(),
            ..Self::default()
        };
        match condition.test {
            Test::Above(above) => data.above = Some(above),
            Test::Below(below) => data.below = Some(below),
            Test::Is(is) => data.is = Some(is),
            Test::Triggered => {}
        }
        data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HERO: &str = r#"
        format = 1
        start = "idle"

        [[value]]
        name = "speed"
        kind = "number"

        [[value]]
        name = "grounded"
        kind = "flag"

        [[value]]
        name = "jump"
        kind = "trigger"

        [[state]]
        name = "idle"
        animation = "idle"
        position = [10.0, 20.0]

        [[state]]
        name = "move"
        blend = "speed"
        points = [{ animation = "walk", at = 1.0 }, { animation = "run", at = 4.0 }]

        [[state]]
        name = "jump"
        animation = "jump"

        [[transition]]
        from = "idle"
        to = "move"
        fade = 0.2
        conditions = [{ value = "speed", above = 0.1 }]

        [[transition]]
        from = "move"
        to = "idle"
        conditions = [{ value = "speed", below = 0.1 }]

        [[transition]]
        to = "jump"
        conditions = [{ value = "jump" }, { value = "grounded", is = true }]

        [[transition]]
        from = "jump"
        to = "idle"
        at_end = true
    "#;

    /// A machine of two states, `a` and `b`, with the values and
    /// transitions of `rest`.
    fn error_of(rest: &str) -> String {
        let text = format!(
            "format = 1\nstart = \"a\"\n{rest}\n\
             [[state]]\nname = \"a\"\nanimation = \"a\"\n\
             [[state]]\nname = \"b\"\nanimation = \"b\"\n"
        );
        AnimationStates::from_text(&text).unwrap_err().to_string()
    }

    #[test]
    fn files_are_read_with_their_values_states_and_transitions() {
        let machine = AnimationStates::from_text(HERO).unwrap();

        assert_eq!(machine.start, "idle");
        assert_eq!(machine.value("grounded").unwrap().kind, ValueKind::Flag);
        let idle = machine.state("idle").unwrap();
        assert_eq!(idle.motion, Motion::Animation("idle".to_owned()));
        assert_eq!(idle.position, [10.0, 20.0]);
        let Motion::Blend { value, points } = &machine.state("move").unwrap().motion else {
            panic!("`move` is a blend");
        };
        assert_eq!(value, "speed");
        assert_eq!((points[1].animation.as_str(), points[1].at), ("run", 4.0));
        let first = &machine.transitions[0];
        assert_eq!(first.from.as_deref(), Some("idle"));
        assert_eq!(first.fade, 0.2);
        assert_eq!(first.conditions[0].test, Test::Above(0.1));
        let jump = &machine.transitions[2];
        assert_eq!(jump.from, None);
        assert_eq!(jump.conditions[0].test, Test::Triggered);
        assert_eq!(jump.conditions[1].test, Test::Is(true));
        assert!(machine.transitions[3].at_end);
    }

    #[test]
    fn files_are_written_and_read_back_the_same() {
        let machine = AnimationStates::from_text(HERO).unwrap();

        let written = machine.to_text().unwrap();

        assert_eq!(AnimationStates::from_text(&written).unwrap(), machine);
    }

    #[test]
    fn the_machine_needs_states_and_a_start_among_them() {
        let empty = AnimationStates::from_text("format = 1\nstart = \"a\"").unwrap_err();
        assert!(empty.to_string().contains("no states"), "{empty}");
        let text = "format = 1\nstart = \"z\"\n[[state]]\nname = \"a\"\nanimation = \"a\"";
        let error = AnimationStates::from_text(text).unwrap_err().to_string();
        assert!(error.contains("starts in \"z\""), "{error}");
        let error = AnimationStates::from_text("format = 2\nstart = \"a\"")
            .unwrap_err()
            .to_string();
        assert!(error.contains("format 2"), "{error}");
    }

    #[test]
    fn values_and_states_need_names_of_their_own() {
        let value = "[[value]]\nname = \"v\"\nkind = \"flag\"\n";
        assert!(error_of(&value.repeat(2)).contains("two values"));
        assert!(error_of("[[value]]\nname = \"\"\nkind = \"flag\"").contains("no name"));
        assert!(error_of("[[state]]\nname = \"a\"\nanimation = \"x\"").contains("two states"));
        let error = error_of("[[state]]\nname = \"c\"\nanimation = \" \"");
        assert!(error.contains("names no animation"), "{error}");
    }

    #[test]
    fn states_play_an_animation_or_a_blend_of_a_number() {
        let both = error_of("[[state]]\nname = \"c\"\nanimation = \"x\"\nblend = \"v\"");
        assert!(both.contains("needs an `animation`"), "{both}");
        let flag = "[[value]]\nname = \"v\"\nkind = \"flag\"\n";
        let error = error_of(&format!(
            "{flag}[[state]]\nname = \"c\"\nblend = \"v\"\npoints = [{{ animation = \"x\", at = 0.0 }}]"
        ));
        assert!(error.contains("is a flag, but a number"), "{error}");
        let number = "[[value]]\nname = \"v\"\nkind = \"number\"\n";
        let error = error_of(&format!("{number}[[state]]\nname = \"c\"\nblend = \"v\""));
        assert!(error.contains("no points"), "{error}");
        let error = error_of(&format!(
            "{number}[[state]]\nname = \"c\"\nblend = \"v\"\n\
             points = [{{ animation = \"x\", at = 2.0 }}, {{ animation = \"y\", at = 1.0 }}]"
        ));
        assert!(error.contains("out of order"), "{error}");
    }

    #[test]
    fn transitions_join_two_states_with_conditions_that_fit() {
        let to_c = error_of("[[transition]]\nfrom = \"a\"\nto = \"c\"");
        assert!(to_c.contains("no state \"c\""), "{to_c}");
        let same = error_of("[[transition]]\nfrom = \"a\"\nto = \"a\"");
        assert!(same.contains("same state"), "{same}");
        let fade = error_of("[[transition]]\nto = \"b\"\nfade = -1.0");
        assert!(fade.contains("zero or more"), "{fade}");
        let missing = error_of("[[transition]]\nto = \"b\"\nconditions = [{ value = \"v\" }]");
        assert!(missing.contains("no value \"v\""), "{missing}");
        let number = "[[value]]\nname = \"v\"\nkind = \"number\"\n";
        let error = error_of(&format!(
            "{number}[[transition]]\nto = \"b\"\nconditions = [{{ value = \"v\", is = true }}]"
        ));
        assert!(error.contains("is a number, but a flag"), "{error}");
        let error = error_of(&format!(
            "{number}[[transition]]\nto = \"b\"\nconditions = [{{ value = \"v\" }}]"
        ));
        assert!(error.contains("but a trigger"), "{error}");
        let error = error_of(&format!(
            "{number}[[transition]]\nto = \"b\"\n\
             conditions = [{{ value = \"v\", above = 1.0, below = 2.0 }}]"
        ));
        assert!(error.contains("one of `above`"), "{error}");
    }
}
