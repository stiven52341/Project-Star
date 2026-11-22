//! What a game declares outside its scripts, in its project settings.

/// An input action of the game, as declared in `project.toml`:
/// `jump = ["Space", "GamepadSouth"]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputAction {
    /// The name scripts use: `Input.is_down("jump")`.
    pub name: String,
    /// The names of its bindings (`"Space"`, `"MouseLeft"`, `"Touch"`,
    /// `"GamepadSouth"`…). The engine checks them when the game starts.
    pub bindings: Vec<String>,
}

/// Project settings that change how scripts are compiled.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GameSettings {
    /// The input actions of the game.
    pub actions: Vec<InputAction>,
}

impl GameSettings {
    /// Whether an action called `name` is declared.
    #[must_use]
    pub fn has_action(&self, name: &str) -> bool {
        self.actions.iter().any(|action| action.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declared_actions_are_found_by_name() {
        let settings = GameSettings {
            actions: vec![InputAction {
                name: "jump".to_owned(),
                bindings: vec!["Space".to_owned()],
            }],
        };

        assert!(settings.has_action("jump"));
        assert!(!settings.has_action("jupm"));
    }
}
