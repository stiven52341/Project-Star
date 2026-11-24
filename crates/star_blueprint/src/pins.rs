//! The pins of each kind of node: what can be wired to what.

use crate::model::{CallTarget, Graph, Node, NodeKind};

/// The execution input of every node that has one.
pub const EXECUTION_INPUT: &str = "in";
/// The execution output of nodes that continue in one way.
pub const EXECUTION_OUTPUT: &str = "then";
/// The execution output a [`NodeKind::Branch`] follows when its condition holds.
pub const BRANCH_TRUE: &str = "true";
/// The execution output a [`NodeKind::Branch`] follows otherwise.
pub const BRANCH_FALSE: &str = "false";
/// The condition of a [`NodeKind::Branch`].
pub const CONDITION: &str = "condition";
/// The value of literals and variables, the value a variable is given, the
/// value returned and the single operand of unary operators.
pub const VALUE: &str = "value";
/// What calls and operators compute.
pub const RESULT: &str = "result";
/// The object whose method a call on a value calls, or whose field a
/// variable node of a value reads or sets.
pub const TARGET: &str = "target";
/// The first operand of binary operators.
pub const LEFT: &str = "left";
/// The second operand of binary operators.
pub const RIGHT: &str = "right";

/// Whether a pin carries the order things happen in, or a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinKind {
    /// The order things happen in.
    Execution,
    /// A value.
    Data,
}

/// Whether a wire ends or starts at a pin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Wires end here.
    Input,
    /// Wires start here.
    Output,
}

/// A pin of a node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pin {
    /// Its name, as links name it.
    pub name: String,
    /// What it carries.
    pub kind: PinKind,
    /// Which end of a wire it is.
    pub direction: Direction,
}

impl Pin {
    fn new(name: &str, kind: PinKind, direction: Direction) -> Self {
        Self {
            name: name.to_owned(),
            kind,
            direction,
        }
    }

    fn data_input(name: &str) -> Self {
        Self::new(name, PinKind::Data, Direction::Input)
    }

    fn data_output(name: &str) -> Self {
        Self::new(name, PinKind::Data, Direction::Output)
    }

    fn execution_input() -> Self {
        Self::new(EXECUTION_INPUT, PinKind::Execution, Direction::Input)
    }

    fn execution_output(name: &str) -> Self {
        Self::new(name, PinKind::Execution, Direction::Output)
    }
}

impl Node {
    /// Its pins, inputs first. Some depend on its `graph`: the entry gives
    /// the graph's parameters and a return takes a value only if the graph
    /// returns one.
    #[must_use]
    pub fn pins(&self, graph: &Graph) -> Vec<Pin> {
        match &self.kind {
            NodeKind::Entry => std::iter::once(Pin::execution_output(EXECUTION_OUTPUT))
                .chain(
                    graph
                        .parameters
                        .iter()
                        .map(|parameter| Pin::data_output(&parameter.name)),
                )
                .collect(),
            NodeKind::Literal { .. }
            | NodeKind::GetVariable {
                of_value: false, ..
            } => {
                vec![Pin::data_output(VALUE)]
            }
            NodeKind::GetVariable { of_value: true, .. } => {
                vec![Pin::data_input(TARGET), Pin::data_output(VALUE)]
            }
            NodeKind::SetVariable { of_value, .. } => {
                let mut pins = vec![Pin::execution_input()];
                if *of_value {
                    pins.push(Pin::data_input(TARGET));
                }
                pins.push(Pin::data_input(VALUE));
                pins.push(Pin::execution_output(EXECUTION_OUTPUT));
                pins
            }
            NodeKind::Call(call) => {
                let mut pins = Vec::new();
                if !call.pure {
                    pins.push(Pin::execution_input());
                }
                if call.target == CallTarget::Value {
                    pins.push(Pin::data_input(TARGET));
                }
                pins.extend(call.arguments.iter().map(|name| Pin::data_input(name)));
                if !call.pure {
                    pins.push(Pin::execution_output(EXECUTION_OUTPUT));
                }
                if call.returns_value {
                    pins.push(Pin::data_output(RESULT));
                }
                pins
            }
            NodeKind::Operator { operator } if operator.is_unary() => {
                vec![Pin::data_input(VALUE), Pin::data_output(RESULT)]
            }
            NodeKind::Operator { .. } => vec![
                Pin::data_input(LEFT),
                Pin::data_input(RIGHT),
                Pin::data_output(RESULT),
            ],
            NodeKind::Branch => vec![
                Pin::execution_input(),
                Pin::data_input(CONDITION),
                Pin::execution_output(BRANCH_TRUE),
                Pin::execution_output(BRANCH_FALSE),
            ],
            NodeKind::Return if graph.returns.is_some() => {
                vec![Pin::execution_input(), Pin::data_input(VALUE)]
            }
            NodeKind::Return => vec![Pin::execution_input()],
        }
    }

    /// The pin called `name`, if the node has one.
    #[must_use]
    pub fn pin(&self, graph: &Graph, name: &str) -> Option<Pin> {
        self.pins(graph).into_iter().find(|pin| pin.name == name)
    }

    /// Whether it runs in the order of the execution wires, rather than
    /// where its value is used.
    #[must_use]
    pub fn is_executed(&self) -> bool {
        match &self.kind {
            NodeKind::Entry
            | NodeKind::SetVariable { .. }
            | NodeKind::Branch
            | NodeKind::Return => true,
            NodeKind::Call(call) => !call.pure,
            NodeKind::Literal { .. } | NodeKind::GetVariable { .. } | NodeKind::Operator { .. } => {
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Call, NodeId, Operator, Parameter};

    fn graph(parameters: &[&str], returns: Option<&str>) -> Graph {
        Graph {
            name: "update".to_owned(),
            parameters: parameters
                .iter()
                .map(|name| Parameter {
                    name: (*name).to_owned(),
                    ty: "float".to_owned(),
                })
                .collect(),
            returns: returns.map(str::to_owned),
            is_override: false,
            private: false,
            nodes: Vec::new(),
            links: Vec::new(),
        }
    }

    fn node(kind: NodeKind) -> Node {
        Node {
            id: NodeId(1),
            position: [0.0, 0.0],
            kind,
        }
    }

    fn names(pins: &[Pin]) -> Vec<&str> {
        pins.iter().map(|pin| pin.name.as_str()).collect()
    }

    #[test]
    fn the_entry_gives_the_parameters() {
        let pins = node(NodeKind::Entry).pins(&graph(&["delta"], None));

        assert_eq!(names(&pins), [EXECUTION_OUTPUT, "delta"]);
        assert_eq!(pins[1].kind, PinKind::Data);
        assert_eq!(pins[1].direction, Direction::Output);
    }

    #[test]
    fn pure_calls_have_no_execution_pins() {
        let call = Call {
            target: CallTarget::Class("Input".to_owned()),
            method: "is_down".to_owned(),
            arguments: vec!["action".to_owned()],
            returns_value: true,
            pure: true,
        };
        let call = node(NodeKind::Call(call));

        assert_eq!(names(&call.pins(&graph(&[], None))), ["action", RESULT]);
        assert!(!call.is_executed());
    }

    #[test]
    fn calls_on_values_take_their_target() {
        let call = Call {
            target: CallTarget::Value,
            method: "hit".to_owned(),
            arguments: Vec::new(),
            returns_value: false,
            pure: false,
        };

        let pins = node(NodeKind::Call(call)).pins(&graph(&[], None));

        assert_eq!(names(&pins), [EXECUTION_INPUT, TARGET, EXECUTION_OUTPUT]);
    }

    #[test]
    fn unary_operators_take_one_value() {
        let not = node(NodeKind::Operator {
            operator: Operator::Not,
        });

        assert_eq!(names(&not.pins(&graph(&[], None))), [VALUE, RESULT]);
    }

    #[test]
    fn returns_take_a_value_only_if_the_graph_returns_one() {
        let ret = node(NodeKind::Return);

        assert_eq!(names(&ret.pins(&graph(&[], None))), [EXECUTION_INPUT]);
        assert_eq!(
            names(&ret.pins(&graph(&[], Some("int")))),
            [EXECUTION_INPUT, VALUE]
        );
    }
}
