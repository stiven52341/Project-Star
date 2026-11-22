//! What a blueprint offers to the nodes of its graphs: the variables they
//! read and write and the methods they call, to pick from instead of
//! typing them.

use star_blueprint::{Blueprint, Call, CallTarget, Literal};

use crate::texts;

/// The names of a blueprint that nodes use.
pub(super) struct BlueprintNames {
    pub(super) variables: Vec<String>,
    pub(super) methods: Vec<Call>,
}

impl BlueprintNames {
    pub(super) fn of(blueprint: &Blueprint) -> Self {
        Self {
            variables: blueprint
                .variables
                .iter()
                .map(|variable| variable.name.clone())
                .collect(),
            methods: blueprint
                .graphs
                .iter()
                .map(|graph| Call {
                    target: CallTarget::OwnObject,
                    method: graph.name.clone(),
                    arguments: graph
                        .parameters
                        .iter()
                        .map(|parameter| parameter.name.clone())
                        .collect(),
                    returns_value: graph.returns.is_some(),
                    pure: false,
                })
                .collect(),
        }
    }
}

/// The kinds of constant a literal node can hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LiteralKind {
    Bool,
    Int,
    Float,
    String,
}

impl LiteralKind {
    pub(super) const ALL: [Self; 4] = [Self::Bool, Self::Int, Self::Float, Self::String];

    pub(super) const fn of(literal: &Literal) -> Self {
        match literal {
            Literal::Bool(_) => Self::Bool,
            Literal::Int(_) => Self::Int,
            Literal::Float(_) => Self::Float,
            Literal::String(_) => Self::String,
        }
    }

    /// Its name in the editor.
    pub(super) const fn name(self) -> &'static str {
        match self {
            Self::Bool => texts::BOOL_KIND,
            Self::Int => texts::INT_KIND,
            Self::Float => texts::FLOAT_KIND,
            Self::String => texts::STRING_KIND,
        }
    }

    /// The value a new constant of this kind starts with.
    pub(super) const fn default_value(self) -> Literal {
        match self {
            Self::Bool => Literal::Bool(false),
            Self::Int => Literal::Int(0),
            Self::Float => Literal::Float(0.0),
            Self::String => Literal::String(String::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use star_blueprint::{Graph, Parameter, Variable};

    use super::*;

    #[test]
    fn methods_are_called_with_the_parameters_of_their_graph() {
        let blueprint = Blueprint {
            class: "Ship".to_owned(),
            extends: None,
            base_arguments: Vec::new(),
            variables: vec![Variable {
                name: "speed".to_owned(),
                ty: None,
                default: Some(Literal::Float(1.0)),
                private: false,
            }],
            graphs: vec![Graph {
                name: "boost".to_owned(),
                parameters: vec![Parameter {
                    name: "amount".to_owned(),
                    ty: "float".to_owned(),
                }],
                returns: Some("bool".to_owned()),
                is_override: false,
                private: false,
                nodes: Vec::new(),
                links: Vec::new(),
            }],
        };

        let names = BlueprintNames::of(&blueprint);

        assert_eq!(names.variables, ["speed"]);
        assert_eq!(names.methods[0].method, "boost");
        assert_eq!(names.methods[0].arguments, ["amount"]);
        assert!(names.methods[0].returns_value);
        assert!(!names.methods[0].pure);
    }

    #[test]
    fn every_kind_of_literal_starts_with_a_value_of_its_kind() {
        for kind in LiteralKind::ALL {
            assert_eq!(LiteralKind::of(&kind.default_value()), kind);
        }
    }
}
