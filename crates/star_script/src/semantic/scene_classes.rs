//! Which classes the nodes of a `.scene` file can be built as.
//!
//! A scene names classes as text (`class = "Rock"`), so the program cannot
//! check them while compiling its scripts. The generated code gives these
//! reasons when a scene is loaded, and the editor when it checks a project.

use std::error::Error;
use std::fmt;

use super::natives::NODE_CLASS;
use super::{ClassId, ClassOrigin, ClassTable};

/// Why a node of a scene cannot be built as the class it names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SceneClassError {
    /// The program has no class with that name.
    Unknown(String),
    /// The class does not extend `Node`.
    NotANode(String),
    /// Its `init` is private, so only the class itself can create objects.
    PrivateInit(String),
    /// Its `init` needs arguments, which a scene cannot give.
    NeedsArguments(String),
}

impl fmt::Display for SceneClassError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown(name) => write!(formatter, "there is no class \"{name}\""),
            Self::NotANode(name) => {
                write!(
                    formatter,
                    "`{name}` is not a node: it does not extend `Node`"
                )
            }
            Self::PrivateInit(name) => write!(
                formatter,
                "`{name}` cannot be created by a scene: its `init` is private"
            ),
            Self::NeedsArguments(name) => write!(
                formatter,
                "`{name}` cannot be created by a scene: its `init` needs arguments"
            ),
        }
    }
}

impl Error for SceneClassError {}

impl ClassTable {
    /// The class a scene node that names `name` is built as.
    ///
    /// # Errors
    ///
    /// Returns why no node can be built as `name`.
    pub fn scene_class(&self, name: &str) -> Result<ClassId, SceneClassError> {
        let class = self
            .find(name)
            .ok_or_else(|| SceneClassError::Unknown(name.to_owned()))?;
        self.check_scene_class(class)?;
        Ok(class)
    }

    /// Whether scene nodes can be built as `class`: a node class created
    /// without arguments. The engine's own node classes always can.
    ///
    /// # Errors
    ///
    /// Returns why they cannot.
    pub fn check_scene_class(&self, class: ClassId) -> Result<(), SceneClassError> {
        let info = self.class(class);
        let name = || info.name.clone();
        let is_node = self
            .find(NODE_CLASS)
            .is_some_and(|node| self.is_subclass(class, node));
        if !is_node {
            return Err(SceneClassError::NotANode(name()));
        }
        if info.origin == ClassOrigin::Native {
            return Ok(());
        }
        if info
            .constructor
            .as_ref()
            .is_some_and(|constructor| constructor.is_private)
        {
            return Err(SceneClassError::PrivateInit(name()));
        }
        if !info.constructor_parameters().is_empty() {
            return Err(SceneClassError::NeedsArguments(name()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check_source;

    fn scene_class(source: &str, name: &str) -> Result<String, SceneClassError> {
        let checked = check_source(source).expect("valid program");
        let classes = checked.analysis.classes();
        classes
            .scene_class(name)
            .map(|class| classes.class(class).name.clone())
    }

    #[test]
    fn node_classes_created_without_arguments_can_be_used() {
        let source = "class Rock extends Sprite {\n  var size = 3\n  init() {\n    super.init(\"Rock\", \"rock.png\")\n  }\n}\n";

        assert_eq!(scene_class(source, "Rock"), Ok("Rock".to_owned()));
    }

    #[test]
    fn engine_node_classes_can_be_used() {
        for name in ["Node", "Sprite", "Camera2D"] {
            assert_eq!(scene_class("", name), Ok(name.to_owned()));
        }
    }

    #[test]
    fn unknown_classes_are_rejected() {
        assert_eq!(
            scene_class("", "Rokc"),
            Err(SceneClassError::Unknown("Rokc".to_owned()))
        );
    }

    #[test]
    fn classes_that_are_not_nodes_are_rejected() {
        assert_eq!(
            scene_class("class Score {}\n", "Score"),
            Err(SceneClassError::NotANode("Score".to_owned()))
        );
        assert_eq!(
            scene_class("", "Color"),
            Err(SceneClassError::NotANode("Color".to_owned()))
        );
    }

    #[test]
    fn classes_that_need_arguments_are_rejected() {
        let source =
            "class Rock extends Node {\n  init(label: string) {\n    super.init(label)\n  }\n}\n";

        assert_eq!(
            scene_class(source, "Rock"),
            Err(SceneClassError::NeedsArguments("Rock".to_owned()))
        );
    }

    #[test]
    fn classes_with_a_private_init_are_rejected() {
        let source =
            "class Rock extends Node {\n  private init() {\n    super.init(\"Rock\")\n  }\n}\n";

        assert_eq!(
            scene_class(source, "Rock"),
            Err(SceneClassError::PrivateInit("Rock".to_owned()))
        );
    }
}
