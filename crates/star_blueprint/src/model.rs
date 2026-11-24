//! What a blueprint is: a class whose methods are node graphs.
//!
//! The model is plain data, like a syntax tree: the visual editor changes it
//! freely, half-built graphs included, and [`validate`](crate::validate)
//! says what is still wrong.

use serde::{Deserialize, Serialize};

/// A class written as node graphs. It translates to one StarScript class.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Blueprint {
    /// Name of the class.
    pub class: String,
    /// Name of the class it extends, if any (`"Sprite"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extends: Option<String>,
    /// The values given to the `init` of the base class, if it needs any:
    /// a node gives its name, and a sprite also its image. The blueprint is
    /// created without arguments.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub base_arguments: Vec<Literal>,
    /// Its fields.
    #[serde(rename = "variable", default, skip_serializing_if = "Vec::is_empty")]
    pub variables: Vec<Variable>,
    /// Its methods, one graph each.
    #[serde(rename = "graph", default, skip_serializing_if = "Vec::is_empty")]
    pub graphs: Vec<Graph>,
}

/// A field of a blueprint: `var speed: float = 200.0`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Variable {
    /// Its name.
    pub name: String,
    /// Its StarScript type (`"float"`, `"[int]"`, `"Node?"`); without one,
    /// it is the type of `default`.
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub ty: Option<String>,
    /// Its starting value; without one, the type's default value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<Literal>,
    /// Whether only the blueprint itself can use it.
    #[serde(default, skip_serializing_if = "is_false")]
    pub private: bool,
}

/// A method of a blueprint, as a graph that starts at its [`NodeKind::Entry`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Graph {
    /// The method's name: an event of the base class (`"update"`) or one of
    /// the blueprint's own.
    pub name: String,
    /// Its parameters, which the entry node gives as values.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<Parameter>,
    /// The StarScript type it returns, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub returns: Option<String>,
    /// Whether it redefines a method of the base class, such as an event.
    #[serde(rename = "override", default, skip_serializing_if = "is_false")]
    pub is_override: bool,
    /// Whether only the blueprint itself can call it.
    #[serde(default, skip_serializing_if = "is_false")]
    pub private: bool,
    /// Its nodes.
    #[serde(rename = "node", default, skip_serializing_if = "Vec::is_empty")]
    pub nodes: Vec<Node>,
    /// The wires between their pins.
    #[serde(rename = "link", default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<Link>,
}

/// A parameter of a [`Graph`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Parameter {
    /// Its name, which is also the name of its pin on the entry node.
    pub name: String,
    /// Its StarScript type.
    #[serde(rename = "type")]
    pub ty: String,
}

/// Identifies a node inside its graph. Links name nodes by it, so it never
/// changes while the node exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeId(pub u32);

/// A node of a graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    /// Its identifier.
    pub id: NodeId,
    /// Where the editor draws it; it does not change what it does.
    #[serde(default)]
    pub position: [f32; 2],
    /// What it does.
    #[serde(flatten)]
    pub kind: NodeKind,
}

/// Every kind of node. [`Node::pins`] lists the pins of each one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NodeKind {
    /// Where the method starts; one per graph. Gives the parameters.
    Entry,
    /// A constant value.
    Literal {
        /// The value.
        value: Literal,
    },
    /// Reads a field of the blueprint (or of its base), or of the object
    /// given to its `target` pin: `enemy.x`.
    GetVariable {
        /// The field's name.
        variable: String,
        /// Whether the field is of the object given to its `target` pin
        /// rather than of the blueprint.
        #[serde(default, skip_serializing_if = "is_false")]
        of_value: bool,
    },
    /// Gives a field a new value: of the blueprint, or of the object given
    /// to its `target` pin.
    SetVariable {
        /// The field's name.
        variable: String,
        /// Whether the field is of the object given to its `target` pin
        /// rather than of the blueprint.
        #[serde(default, skip_serializing_if = "is_false")]
        of_value: bool,
    },
    /// Calls a method.
    Call(Call),
    /// An operator applied to one or two values.
    Operator {
        /// The operator.
        operator: Operator,
    },
    /// Goes one way or the other depending on a condition: an `if`.
    Branch,
    /// Ends the method, giving back a value if the method returns one.
    Return,
}

/// A call of a method, as a [`NodeKind::Call`] node makes it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Call {
    /// The object or class whose method is called.
    pub target: CallTarget,
    /// The method's name.
    pub method: String,
    /// The names of its arguments, in order; each one is an input pin.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub arguments: Vec<String>,
    /// Whether it gives back a value, as its `result` pin.
    #[serde(default, skip_serializing_if = "is_false")]
    pub returns_value: bool,
    /// Whether it only computes a value (`Input.is_down`): then it has no
    /// execution pins and runs where its value is used.
    #[serde(default, skip_serializing_if = "is_false")]
    pub pure: bool,
}

/// What a [`Call`] calls a method of.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CallTarget {
    /// The blueprint's own object: `jump()`.
    #[serde(rename = "self")]
    OwnObject,
    /// The object given to its `target` pin: `enemy.hit()`.
    Value,
    /// A class, for its static methods: `Input.is_down(…)`.
    Class(String),
}

/// A constant value written in a blueprint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Literal {
    /// `true` or `false`.
    Bool(bool),
    /// A whole number.
    Int(i64),
    /// A number with decimals.
    Float(f64),
    /// A text.
    String(String),
}

/// The operators an [`NodeKind::Operator`] node can apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operator {
    /// `left + right`
    Add,
    /// `left - right`
    Subtract,
    /// `left * right`
    Multiply,
    /// `left / right`
    Divide,
    /// `left % right`
    Remainder,
    /// `left == right`
    Equal,
    /// `left != right`
    NotEqual,
    /// `left < right`
    Less,
    /// `left <= right`
    LessOrEqual,
    /// `left > right`
    Greater,
    /// `left >= right`
    GreaterOrEqual,
    /// `left && right`
    And,
    /// `left || right`
    Or,
    /// `!value`
    Not,
    /// `-value`
    Negate,
}

impl Operator {
    /// Every operator, in the order an editor lists them.
    pub const ALL: [Self; 15] = [
        Self::Add,
        Self::Subtract,
        Self::Multiply,
        Self::Divide,
        Self::Remainder,
        Self::Negate,
        Self::Equal,
        Self::NotEqual,
        Self::Less,
        Self::LessOrEqual,
        Self::Greater,
        Self::GreaterOrEqual,
        Self::And,
        Self::Or,
        Self::Not,
    ];

    /// How it is written in StarScript.
    #[must_use]
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Subtract | Self::Negate => "-",
            Self::Multiply => "*",
            Self::Divide => "/",
            Self::Remainder => "%",
            Self::Equal => "==",
            Self::NotEqual => "!=",
            Self::Less => "<",
            Self::LessOrEqual => "<=",
            Self::Greater => ">",
            Self::GreaterOrEqual => ">=",
            Self::And => "&&",
            Self::Or => "||",
            Self::Not => "!",
        }
    }

    /// Whether it takes one value instead of two.
    #[must_use]
    pub const fn is_unary(self) -> bool {
        matches!(self, Self::Not | Self::Negate)
    }
}

/// A wire from an output pin to an input pin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Link {
    /// The output pin it starts at.
    pub from: PinRef,
    /// The input pin it ends at.
    pub to: PinRef,
}

/// A pin of a node of the same graph.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PinRef {
    /// The node.
    pub node: NodeId,
    /// The pin's name on that node.
    pub pin: String,
}

impl PinRef {
    /// The pin called `pin` of `node`.
    #[must_use]
    pub fn new(node: NodeId, pin: &str) -> Self {
        Self {
            node,
            pin: pin.to_owned(),
        }
    }
}

impl Graph {
    /// The node with the identifier `id`.
    #[must_use]
    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.iter().find(|node| node.id == id)
    }
}

#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde passes the field by reference"
)]
fn is_false(value: &bool) -> bool {
    !*value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_operator_is_listed_once() {
        for (index, operator) in Operator::ALL.iter().enumerate() {
            assert!(
                !Operator::ALL[..index].contains(operator),
                "{operator:?} is listed twice"
            );
        }
    }
}
