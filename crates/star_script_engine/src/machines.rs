//! `.states` files of the game's asset folder, read for the animators
//! whose state machines choose their animations (decision 81).

use star_engine::star_assets::{AssetFolder, AssetPath};
use star_engine::star_scene::{
    AnimationState, AnimationStates, Condition, Motion, StateTransition, Test, ValueKind,
};
use star_script_runtime::machines::{self, BlendPoint, MachineState, MachineValue, StateMachine};

/// Reads the `.states` file at `path` of the asset `folder`.
///
/// # Errors
///
/// Why the file could not be read, for the warning the game writes.
pub fn read_states(folder: &AssetFolder, path: &str) -> Result<StateMachine, String> {
    let file = AssetPath::new(path)
        .and_then(|path| folder.read_text(&path))
        .and_then(|text| AnimationStates::from_text(&text))
        .map_err(|error| error.to_string())?;
    Ok(state_machine(file))
}

/// The runtime's machine of a file's.
#[must_use]
pub fn state_machine(file: AnimationStates) -> StateMachine {
    StateMachine {
        values: file
            .values
            .into_iter()
            .map(|value| MachineValue {
                name: value.name,
                kind: value_kind(value.kind),
            })
            .collect(),
        start: file.start,
        states: file.states.into_iter().map(machine_state).collect(),
        transitions: file.transitions.into_iter().map(transition).collect(),
    }
}

const fn value_kind(kind: ValueKind) -> machines::ValueKind {
    match kind {
        ValueKind::Number => machines::ValueKind::Number,
        ValueKind::Flag => machines::ValueKind::Flag,
        ValueKind::Trigger => machines::ValueKind::Trigger,
    }
}

fn machine_state(state: AnimationState) -> MachineState {
    let motion = match state.motion {
        Motion::Animation(animation) => machines::Motion::Animation(animation),
        Motion::Blend { value, points } => machines::Motion::Blend {
            value,
            points: points
                .into_iter()
                .map(|point| BlendPoint {
                    animation: point.animation,
                    at: point.at,
                })
                .collect(),
        },
    };
    MachineState {
        name: state.name,
        motion,
    }
}

fn transition(transition: StateTransition) -> machines::Transition {
    machines::Transition {
        from: transition.from,
        to: transition.to,
        conditions: transition.conditions.into_iter().map(condition).collect(),
        fade: transition.fade,
        at_end: transition.at_end,
    }
}

fn condition(condition: Condition) -> machines::Condition {
    machines::Condition {
        value: condition.value,
        test: match condition.test {
            Test::Above(limit) => machines::Test::Above(limit),
            Test::Below(limit) => machines::Test::Below(limit),
            Test::Is(value) => machines::Test::Is(value),
            Test::Triggered => machines::Test::Triggered,
        },
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

        [[state]]
        name = "idle"
        animation = "idle"

        [[state]]
        name = "move"
        blend = "speed"
        points = [{ animation = "walk", at = 1.0 }, { animation = "run", at = 3.0 }]

        [[transition]]
        from = "idle"
        to = "move"
        fade = 0.25
        conditions = [{ value = "speed", above = 0.5 }]
    "#;

    #[test]
    fn files_become_the_runtime_machines() {
        let machine = state_machine(AnimationStates::from_text(HERO).unwrap());

        assert_eq!(machine.start, "idle");
        assert_eq!(machine.values[0].kind, machines::ValueKind::Number);
        let machines::Motion::Blend { value, points } = &machine.states[1].motion else {
            panic!("`move` blends");
        };
        assert_eq!(value, "speed");
        assert_eq!(points[1].animation, "run");
        let first = &machine.transitions[0];
        assert_eq!(first.from.as_deref(), Some("idle"));
        assert_eq!(first.fade, 0.25);
        assert_eq!(first.conditions[0].test, machines::Test::Above(0.5));
    }
}
