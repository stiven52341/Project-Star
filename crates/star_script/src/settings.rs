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

/// The scene a game without a `Game` class plays.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MainScene {
    /// Its path in the asset folder: `scenes/level.scene`.
    pub path: String,
    /// The title of the game's window.
    pub title: String,
}

/// When a game shows its frames-per-second counter from the start
/// (decision 85): `show_fps` in `project.toml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShowFps {
    /// When the editor plays it (“Play” and “Play compiled”), not once it
    /// is exported.
    #[default]
    Editor,
    /// Always, exported games too.
    Always,
    /// Never: only F3 or `Debug.show_fps` show it.
    Never,
}

impl ShowFps {
    /// Every choice, in the order they are offered.
    pub const ALL: [Self; 3] = [Self::Editor, Self::Always, Self::Never];

    /// The word `project.toml` writes it with.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Editor => "editor",
            Self::Always => "always",
            Self::Never => "never",
        }
    }

    /// The choice written as `name`, if it is one.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|choice| choice.name() == name)
    }
}

/// Project settings that change how scripts are compiled.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GameSettings {
    /// The input actions of the game.
    pub actions: Vec<InputAction>,
    /// What the game plays when no class extends `Game`, if anything.
    pub main_scene: Option<MainScene>,
    /// When the game shows its frames per second from the start.
    pub show_fps: ShowFps,
    /// The groups of sounds and their volumes when the game starts
    /// (decision 87), by name.
    pub sound_groups: Vec<(String, f32)>,
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
    fn fps_choices_are_written_with_their_names() {
        for choice in ShowFps::ALL {
            assert_eq!(ShowFps::from_name(choice.name()), Some(choice));
        }
        assert_eq!(ShowFps::from_name("sometimes"), None);
        assert_eq!(ShowFps::default(), ShowFps::Editor);
    }

    #[test]
    fn declared_actions_are_found_by_name() {
        let settings = GameSettings {
            actions: vec![InputAction {
                name: "jump".to_owned(),
                bindings: vec!["Space".to_owned()],
            }],
            main_scene: None,
            ..Default::default()
        };

        assert!(settings.has_action("jump"));
        assert!(!settings.has_action("jupm"));
    }
}
