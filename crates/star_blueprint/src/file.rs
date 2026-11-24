//! `.blueprint` files: TOML, with the version of their layout.

use std::error::Error;
use std::fmt;

use crate::model::Blueprint;

/// The version of the layout this engine writes and reads.
pub const FORMAT_VERSION: u32 = 1;

/// Key of the layout version, at the top of the file.
const FORMAT_KEY: &str = "format";

/// Why a text is not a `.blueprint` file this engine can read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlueprintFileError {
    /// It is not valid TOML, or it does not have the shape of a blueprint.
    Syntax(String),
    /// It has no `format`, or one this engine does not know.
    UnsupportedFormat(Option<i64>),
    /// The blueprint cannot be written as TOML.
    Write(String),
}

impl fmt::Display for BlueprintFileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Syntax(reason) | Self::Write(reason) => formatter.write_str(reason),
            Self::UnsupportedFormat(None) => {
                write!(formatter, "the file has no `{FORMAT_KEY}` version")
            }
            Self::UnsupportedFormat(Some(version)) => write!(
                formatter,
                "unknown format version {version} (this engine reads {FORMAT_VERSION})"
            ),
        }
    }
}

impl Error for BlueprintFileError {}

impl Blueprint {
    /// Reads the text of a `.blueprint` file. The rules of its graphs are
    /// not checked (see [`validate`](crate::validate)), so the editor can
    /// open blueprints that are still being built.
    ///
    /// # Errors
    ///
    /// Returns why the text is not a blueprint of this format version.
    pub fn from_text(text: &str) -> Result<Self, BlueprintFileError> {
        let mut table: toml::Table = text.parse().map_err(|error: toml::de::Error| {
            BlueprintFileError::Syntax(error.message().to_owned())
        })?;
        match table.remove(FORMAT_KEY) {
            Some(toml::Value::Integer(version)) if version == i64::from(FORMAT_VERSION) => {}
            Some(toml::Value::Integer(version)) => {
                return Err(BlueprintFileError::UnsupportedFormat(Some(version)));
            }
            _ => return Err(BlueprintFileError::UnsupportedFormat(None)),
        }
        toml::Value::Table(table)
            .try_into()
            .map_err(|error: toml::de::Error| {
                BlueprintFileError::Syntax(error.message().to_owned())
            })
    }

    /// The text of its `.blueprint` file.
    ///
    /// # Errors
    ///
    /// Returns [`BlueprintFileError::Write`] if a value cannot be written in
    /// TOML (a number that is not finite).
    pub fn to_text(&self) -> Result<String, BlueprintFileError> {
        let write_error = |error: toml::ser::Error| BlueprintFileError::Write(error.to_string());
        let body = toml::to_string(self).map_err(write_error)?;
        Ok(format!("{FORMAT_KEY} = {FORMAT_VERSION}\n{body}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        Call, CallTarget, Graph, Link, Literal, Node, NodeId, NodeKind, Parameter, PinRef, Variable,
    };
    use crate::pins::{EXECUTION_INPUT, EXECUTION_OUTPUT, VALUE};

    /// The example of `docs/starscript.md`, section 5 (keep both equal).
    const DOCUMENTED_EXAMPLE: &str = include_str!("../examples/player.blueprint");

    fn sample() -> Blueprint {
        Blueprint {
            class: "Player".to_owned(),
            extends: Some("Sprite".to_owned()),
            base_arguments: vec![Literal::String("Player".to_owned())],
            variables: vec![
                Variable {
                    name: "speed".to_owned(),
                    ty: Some("float".to_owned()),
                    default: Some(Literal::Float(2.5)),
                    private: false,
                    export: false,
                },
                Variable {
                    name: "lives".to_owned(),
                    ty: None,
                    default: Some(Literal::Int(3)),
                    private: true,
                    export: false,
                },
            ],
            graphs: vec![Graph {
                name: "update".to_owned(),
                parameters: vec![Parameter {
                    name: "delta".to_owned(),
                    ty: "float".to_owned(),
                }],
                returns: None,
                is_override: true,
                private: false,
                nodes: vec![
                    Node {
                        id: NodeId(1),
                        position: [0.0, 0.0],
                        kind: NodeKind::Entry,
                    },
                    Node {
                        id: NodeId(2),
                        position: [200.0, 40.0],
                        kind: NodeKind::Call(Call {
                            target: CallTarget::Class("Input".to_owned()),
                            method: "is_down".to_owned(),
                            arguments: vec!["action".to_owned()],
                            returns_value: true,
                            pure: true,
                        }),
                    },
                    Node {
                        id: NodeId(3),
                        position: [0.0, 120.0],
                        kind: NodeKind::Literal {
                            value: Literal::String("jump".to_owned()),
                        },
                    },
                    Node {
                        id: NodeId(4),
                        position: [400.0, 0.0],
                        kind: NodeKind::Call(Call {
                            target: CallTarget::OwnObject,
                            method: "jump".to_owned(),
                            arguments: Vec::new(),
                            returns_value: false,
                            pure: false,
                        }),
                    },
                ],
                links: vec![
                    Link {
                        from: PinRef::new(NodeId(3), VALUE),
                        to: PinRef::new(NodeId(2), "action"),
                    },
                    Link {
                        from: PinRef::new(NodeId(1), EXECUTION_OUTPUT),
                        to: PinRef::new(NodeId(4), EXECUTION_INPUT),
                    },
                ],
            }],
        }
    }

    #[test]
    fn blueprints_survive_being_written_and_read() {
        let blueprint = sample();

        let text = blueprint.to_text().expect("writable");

        assert!(text.starts_with("format = 1\n"), "{text}");
        assert_eq!(Blueprint::from_text(&text), Ok(blueprint));
    }

    #[test]
    fn files_are_readable_by_hand() {
        let text = "format = 1\nclass = \"Rock\"\n\n\
                    [[graph]]\nname = \"start\"\noverride = true\n\n\
                    [[graph.node]]\nid = 1\nkind = \"entry\"\n\n\
                    [[graph.node]]\nid = 2\nkind = \"call\"\ntarget = { class = \"Debug\" }\n\
                    method = \"print\"\narguments = [\"text\"]\n";

        let blueprint = Blueprint::from_text(text).expect("valid file");

        let graph = &blueprint.graphs[0];
        assert!(graph.is_override);
        assert_eq!(graph.nodes[1].position, [0.0, 0.0]);
        assert!(matches!(
            &graph.nodes[1].kind,
            NodeKind::Call(Call { target: CallTarget::Class(class), .. }) if class == "Debug"
        ));
    }

    #[test]
    fn variables_of_a_value_are_marked_only_when_they_are() {
        let text = "format = 1\nclass = \"A\"\n\n\
                    [[graph]]\nname = \"start\"\n\n\
                    [[graph.node]]\nid = 1\nkind = \"get_variable\"\nvariable = \"x\"\n\n\
                    [[graph.node]]\nid = 2\nkind = \"set_variable\"\nvariable = \"x\"\n\
                    of_value = true\n";

        let blueprint = Blueprint::from_text(text).expect("valid file");
        let written = blueprint.to_text().expect("writable");

        let nodes = &blueprint.graphs[0].nodes;
        assert!(matches!(
            nodes[0].kind,
            NodeKind::GetVariable {
                of_value: false,
                ..
            }
        ));
        assert!(matches!(
            nodes[1].kind,
            NodeKind::SetVariable { of_value: true, .. }
        ));
        assert_eq!(written.matches("of_value").count(), 1, "{written}");
    }

    #[test]
    fn whole_numbers_and_decimals_stay_apart() {
        let text = "format = 1\nclass = \"A\"\n\n[[variable]]\nname = \"a\"\ndefault = 3\n\n\
                    [[variable]]\nname = \"b\"\ndefault = 3.0\n";

        let blueprint = Blueprint::from_text(text).expect("valid file");

        assert_eq!(blueprint.variables[0].default, Some(Literal::Int(3)));
        assert_eq!(blueprint.variables[1].default, Some(Literal::Float(3.0)));
    }

    #[test]
    fn the_example_of_the_documentation_is_valid() {
        let blueprint = Blueprint::from_text(DOCUMENTED_EXAMPLE).expect("valid file");

        assert_eq!(crate::validate(&blueprint), Ok(()));
    }

    #[test]
    fn other_format_versions_are_rejected() {
        assert_eq!(
            Blueprint::from_text("format = 2\nclass = \"A\"\n"),
            Err(BlueprintFileError::UnsupportedFormat(Some(2)))
        );
        assert_eq!(
            Blueprint::from_text("class = \"A\"\n"),
            Err(BlueprintFileError::UnsupportedFormat(None))
        );
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let result = Blueprint::from_text("format = 1\nclass = \"A\"\nextend = \"Node\"\n");

        assert!(
            matches!(result, Err(BlueprintFileError::Syntax(_))),
            "{result:?}"
        );
    }
}
