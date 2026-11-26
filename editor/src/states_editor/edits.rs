//! Changes to a state machine that keep it joined together: renaming a
//! state renames it in its transitions, deleting a value deletes the
//! conditions on it, and so on.

use star_engine::star_scene::{
    AnimationState, AnimationStates, BlendPoint, Condition, Motion, StateTransition, StateValue,
    Test, ValueKind,
};

/// What new states are called: `state1`, `state2`…
const NEW_STATE_PREFIX: &str = "state";
/// What new values are called: `value1`, `value2`…
const NEW_VALUE_PREFIX: &str = "value";
/// How far apart, in the value, a new point of a blend is from the last.
const BLEND_POINT_STEP: f32 = 1.0;

/// The first of `prefix1`, `prefix2`… that `taken` says is free.
fn free_name(prefix: &str, taken: impl Fn(&str) -> bool) -> String {
    (1..=u32::MAX)
        .map(|number| format!("{prefix}{number}"))
        .find(|name| !taken(name))
        .unwrap_or_else(|| prefix.to_owned())
}

/// Adds a state at `position` of the graph, playing `animation`. Returns
/// its index.
pub(super) fn add_state(
    machine: &mut AnimationStates,
    position: [f32; 2],
    animation: &str,
) -> usize {
    let name = free_name(NEW_STATE_PREFIX, |name| machine.state(name).is_some());
    machine.states.push(AnimationState {
        name,
        motion: Motion::Animation(animation.to_owned()),
        position,
    });
    machine.states.len() - 1
}

/// Renames the state at `index` to `name`, in the transitions and the
/// start too. Returns whether it did: a name must not be empty nor
/// another state's.
pub(super) fn rename_state(machine: &mut AnimationStates, index: usize, name: &str) -> bool {
    let taken = machine
        .states
        .iter()
        .enumerate()
        .any(|(other, state)| other != index && state.name == name);
    let Some(state) = machine.states.get_mut(index) else {
        return false;
    };
    if name.trim().is_empty() || taken {
        return false;
    }
    let old = std::mem::replace(&mut state.name, name.to_owned());
    if machine.start == old {
        name.clone_into(&mut machine.start);
    }
    for transition in &mut machine.transitions {
        if transition.from.as_deref() == Some(old.as_str()) {
            transition.from = Some(name.to_owned());
        }
        if transition.to == old {
            name.clone_into(&mut transition.to);
        }
    }
    true
}

/// Deletes the state at `index` and its transitions; if it was the start,
/// the first state left starts. Returns whether it did: the last state
/// stays.
pub(super) fn delete_state(machine: &mut AnimationStates, index: usize) -> bool {
    if machine.states.len() <= 1 || index >= machine.states.len() {
        return false;
    }
    let state = machine.states.remove(index);
    machine.transitions.retain(|transition| {
        transition.to != state.name && transition.from.as_deref() != Some(state.name.as_str())
    });
    if machine.start == state.name
        && let Some(first) = machine.states.first()
    {
        first.name.clone_into(&mut machine.start);
    }
    true
}

/// Adds a transition from `from` (or any state) to `to`, without
/// conditions nor fade. Returns its index, or `None` if it would leave
/// and enter the same state.
pub(super) fn add_transition(
    machine: &mut AnimationStates,
    from: Option<&str>,
    to: &str,
) -> Option<usize> {
    if from == Some(to) {
        return None;
    }
    machine.transitions.push(StateTransition {
        from: from.map(str::to_owned),
        to: to.to_owned(),
        conditions: Vec::new(),
        fade: 0.0,
        at_end: false,
    });
    Some(machine.transitions.len() - 1)
}

/// Adds a value of `kind`. Returns its index.
pub(super) fn add_value(machine: &mut AnimationStates, kind: ValueKind) -> usize {
    let name = free_name(NEW_VALUE_PREFIX, |name| machine.value(name).is_some());
    machine.values.push(StateValue { name, kind });
    machine.values.len() - 1
}

/// Renames the value at `index` to `name`, in the conditions and blends
/// too. Returns whether it did: a name must not be empty nor another
/// value's.
pub(super) fn rename_value(machine: &mut AnimationStates, index: usize, name: &str) -> bool {
    let taken = machine
        .values
        .iter()
        .enumerate()
        .any(|(other, value)| other != index && value.name == name);
    let Some(value) = machine.values.get_mut(index) else {
        return false;
    };
    if name.trim().is_empty() || taken {
        return false;
    }
    let old = std::mem::replace(&mut value.name, name.to_owned());
    for condition in conditions_mut(machine) {
        if condition.value == old {
            name.clone_into(&mut condition.value);
        }
    }
    for state in &mut machine.states {
        if let Motion::Blend { value, .. } = &mut state.motion
            && *value == old
        {
            name.clone_into(value);
        }
    }
    true
}

/// Deletes the value at `index` and the conditions on it. Blends by it
/// stay, for the user to choose another value.
pub(super) fn delete_value(machine: &mut AnimationStates, index: usize) {
    if index >= machine.values.len() {
        return;
    }
    let value = machine.values.remove(index);
    for transition in &mut machine.transitions {
        transition
            .conditions
            .retain(|condition| condition.value != value.name);
    }
}

/// Makes the value at `index` hold `kind`, and the conditions on it test
/// what a value of that kind can.
pub(super) fn set_value_kind(machine: &mut AnimationStates, index: usize, kind: ValueKind) {
    let Some(value) = machine.values.get_mut(index) else {
        return;
    };
    value.kind = kind;
    let name = value.name.clone();
    for condition in conditions_mut(machine) {
        if condition.value == name && kind_tested(condition.test) != kind {
            condition.test = default_test(kind);
        }
    }
}

/// A condition on the first value, or `None` if there are no values.
pub(super) fn new_condition(machine: &AnimationStates) -> Option<Condition> {
    let value = machine.values.first()?;
    Some(Condition {
        value: value.name.clone(),
        test: default_test(value.kind),
    })
}

/// The test a new condition on a value of `kind` makes.
pub(super) const fn default_test(kind: ValueKind) -> Test {
    match kind {
        ValueKind::Number => Test::Above(0.0),
        ValueKind::Flag => Test::Is(true),
        ValueKind::Trigger => Test::Triggered,
    }
}

/// The kind of value `test` tests.
pub(super) const fn kind_tested(test: Test) -> ValueKind {
    match test {
        Test::Above(_) | Test::Below(_) => ValueKind::Number,
        Test::Is(_) => ValueKind::Flag,
        Test::Triggered => ValueKind::Trigger,
    }
}

/// A point after the last of `points`, playing its animation.
pub(super) fn next_point(points: &[BlendPoint]) -> BlendPoint {
    points.last().map_or_else(
        || BlendPoint {
            animation: String::new(),
            at: 0.0,
        },
        |last| BlendPoint {
            animation: last.animation.clone(),
            at: last.at + BLEND_POINT_STEP,
        },
    )
}

fn conditions_mut(machine: &mut AnimationStates) -> impl Iterator<Item = &mut Condition> {
    machine
        .transitions
        .iter_mut()
        .flat_map(|transition| transition.conditions.iter_mut())
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
        points = [{ animation = "walk", at = 1.0 }]

        [[transition]]
        from = "idle"
        to = "move"
        conditions = [{ value = "speed", above = 0.1 }]

        [[transition]]
        to = "idle"
    "#;

    fn hero() -> AnimationStates {
        AnimationStates::from_text(HERO).unwrap()
    }

    #[test]
    fn new_states_and_values_get_free_names() {
        let mut machine = hero();

        let first = add_state(&mut machine, [10.0, 20.0], "walk");
        let second = add_state(&mut machine, [0.0, 0.0], "walk");
        let value = add_value(&mut machine, ValueKind::Flag);

        assert_eq!(machine.states[first].name, "state1");
        assert_eq!(machine.states[first].position, [10.0, 20.0]);
        assert_eq!(machine.states[second].name, "state2");
        assert_eq!(machine.values[value].name, "value1");
        machine.validate().unwrap();
    }

    #[test]
    fn renaming_a_state_renames_it_everywhere() {
        let mut machine = hero();

        assert!(rename_state(&mut machine, 0, "rest"));

        assert_eq!(machine.start, "rest");
        assert_eq!(machine.transitions[0].from.as_deref(), Some("rest"));
        assert_eq!(machine.transitions[1].to, "rest");
        assert!(!rename_state(&mut machine, 0, "move"), "taken");
        assert!(!rename_state(&mut machine, 0, " "), "empty");
        machine.validate().unwrap();
    }

    #[test]
    fn deleting_a_state_deletes_its_transitions_and_keeps_a_start() {
        let mut machine = hero();

        assert!(delete_state(&mut machine, 0));

        assert_eq!(machine.start, "move");
        assert_eq!(machine.transitions, Vec::new());
        assert!(!delete_state(&mut machine, 0), "the last state stays");
        machine.validate().unwrap();
    }

    #[test]
    fn transitions_join_two_different_states() {
        let mut machine = hero();

        assert_eq!(add_transition(&mut machine, Some("move"), "move"), None);
        let index = add_transition(&mut machine, Some("move"), "idle").unwrap();

        assert_eq!(machine.transitions[index].to, "idle");
        machine.validate().unwrap();
    }

    #[test]
    fn renaming_a_value_renames_its_conditions_and_blends() {
        let mut machine = hero();

        assert!(rename_value(&mut machine, 0, "pace"));

        assert_eq!(machine.transitions[0].conditions[0].value, "pace");
        let Motion::Blend { value, .. } = &machine.states[1].motion else {
            panic!("`move` blends");
        };
        assert_eq!(value, "pace");
        machine.validate().unwrap();
    }

    #[test]
    fn deleting_a_value_deletes_its_conditions() {
        let mut machine = hero();

        delete_value(&mut machine, 0);

        assert_eq!(machine.values, Vec::new());
        assert_eq!(machine.transitions[0].conditions, Vec::new());
    }

    #[test]
    fn values_of_another_kind_get_conditions_they_can_take() {
        let mut machine = hero();

        set_value_kind(&mut machine, 0, ValueKind::Flag);

        assert_eq!(machine.transitions[0].conditions[0].test, Test::Is(true));
    }

    #[test]
    fn new_blend_points_go_after_the_last() {
        let points = [BlendPoint {
            animation: "walk".to_owned(),
            at: 1.0,
        }];

        let next = next_point(&points);

        assert_eq!((next.animation.as_str(), next.at), ("walk", 2.0));
        assert_eq!(next_point(&[]).at, 0.0);
    }
}
