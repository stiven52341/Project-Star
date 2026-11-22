//! Configuration of the game's actions: which inputs trigger each one.

use star_core::{EngineError, EngineResult};

use crate::binding::Binding;

/// One action and the inputs that trigger it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionBindings {
    pub(crate) name: String,
    pub(crate) bindings: Vec<Binding>,
}

/// The actions a game understands (e.g. `"jump"`) and the inputs bound to
/// each one, so game code never asks for a specific device.
///
/// Every action has a unique, non-empty name and at least one binding.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ActionMap {
    actions: Vec<ActionBindings>,
}

impl ActionMap {
    /// A map without actions.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            actions: Vec::new(),
        }
    }

    /// Adds the action `name`, held while any of `bindings` is held.
    /// A binding listed twice counts once.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Input`] if `name` is empty, an action with that
    /// name already exists, or `bindings` is empty.
    pub fn add(&mut self, name: &str, bindings: &[Binding]) -> EngineResult<()> {
        if name.is_empty() {
            return Err(EngineError::Input("an action needs a name".to_owned()));
        }
        if self.contains(name) {
            return Err(EngineError::Input(format!(
                "the action \"{name}\" already exists"
            )));
        }
        if bindings.is_empty() {
            return Err(EngineError::Input(format!(
                "the action \"{name}\" needs at least one binding"
            )));
        }
        self.actions.push(ActionBindings {
            name: name.to_owned(),
            bindings: without_duplicates(bindings),
        });
        Ok(())
    }

    /// Whether an action called `name` exists.
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.actions.iter().any(|action| action.name == name)
    }

    pub(crate) fn into_actions(self) -> Vec<ActionBindings> {
        self.actions
    }
}

fn without_duplicates(bindings: &[Binding]) -> Vec<Binding> {
    let mut unique = Vec::with_capacity(bindings.len());
    for binding in bindings {
        if !unique.contains(binding) {
            unique.push(*binding);
        }
    }
    unique
}

#[cfg(test)]
mod tests {
    use star_platform::{Key, MouseButton};

    use super::*;

    const JUMP: &str = "jump";

    #[test]
    fn added_actions_exist() {
        let mut map = ActionMap::new();

        map.add(JUMP, &[Key::Space.into(), MouseButton::Left.into()])
            .unwrap();

        assert!(map.contains(JUMP));
        assert!(!map.contains("fire"));
    }

    #[test]
    fn action_names_cannot_be_empty() {
        let result = ActionMap::new().add("", &[Key::Space.into()]);

        assert!(matches!(result, Err(EngineError::Input(_))));
    }

    #[test]
    fn action_names_are_unique() {
        let mut map = ActionMap::new();
        map.add(JUMP, &[Key::Space.into()]).unwrap();

        let result = map.add(JUMP, &[Key::W.into()]);

        assert!(matches!(result, Err(EngineError::Input(_))));
    }

    #[test]
    fn actions_need_a_binding() {
        let result = ActionMap::new().add(JUMP, &[]);

        assert!(matches!(result, Err(EngineError::Input(_))));
    }

    #[test]
    fn repeated_bindings_count_once() {
        let mut map = ActionMap::new();

        map.add(JUMP, &[Key::Space.into(), Key::Space.into()])
            .unwrap();

        assert_eq!(map.into_actions()[0].bindings, [Binding::Key(Key::Space)]);
    }
}
