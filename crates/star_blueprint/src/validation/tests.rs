use super::*;
use crate::model::{Call, Literal, Node, Operator, Parameter, PinRef, Variable};
use crate::pins::{
    BRANCH_FALSE, BRANCH_TRUE, CONDITION, EXECUTION_OUTPUT, LEFT, RESULT, RIGHT, VALUE,
};

/// A graph `update(delta: float)` built node by node.
struct GraphBuilder {
    graph: Graph,
}

impl GraphBuilder {
    fn new() -> Self {
        Self {
            graph: Graph {
                name: "update".to_owned(),
                parameters: vec![Parameter {
                    name: "delta".to_owned(),
                    ty: "float".to_owned(),
                }],
                returns: None,
                is_override: true,
                private: false,
                nodes: Vec::new(),
                links: Vec::new(),
            },
        }
    }

    fn node(mut self, id: u32, kind: NodeKind) -> Self {
        self.graph.nodes.push(Node {
            id: NodeId(id),
            position: [0.0, 0.0],
            kind,
        });
        self
    }

    fn link(mut self, from: (u32, &str), to: (u32, &str)) -> Self {
        self.graph.links.push(Link {
            from: PinRef::new(NodeId(from.0), from.1),
            to: PinRef::new(NodeId(to.0), to.1),
        });
        self
    }

    fn errors(self) -> Vec<(Option<u32>, BlueprintErrorKind)> {
        blueprint_errors(&blueprint(vec![self.graph]))
    }
}

fn blueprint(graphs: Vec<Graph>) -> Blueprint {
    Blueprint {
        class: "Player".to_owned(),
        extends: Some("Sprite".to_owned()),
        variables: vec![Variable {
            name: "speed".to_owned(),
            ty: None,
            default: Some(Literal::Float(2.0)),
            private: false,
        }],
        graphs,
    }
}

fn blueprint_errors(blueprint: &Blueprint) -> Vec<(Option<u32>, BlueprintErrorKind)> {
    validate(blueprint)
        .err()
        .unwrap_or_default()
        .into_iter()
        .map(|error| (error.node.map(|NodeId(id)| id), error.kind))
        .collect()
}

fn set_speed() -> NodeKind {
    NodeKind::SetVariable {
        variable: "speed".to_owned(),
    }
}

fn get_speed() -> NodeKind {
    NodeKind::GetVariable {
        variable: "speed".to_owned(),
    }
}

fn operator(operator: Operator) -> NodeKind {
    NodeKind::Operator { operator }
}

/// `speed = speed * delta`
fn scale_speed() -> GraphBuilder {
    GraphBuilder::new()
        .node(1, NodeKind::Entry)
        .node(2, set_speed())
        .node(3, get_speed())
        .node(4, operator(Operator::Multiply))
        .link((1, EXECUTION_OUTPUT), (2, EXECUTION_INPUT))
        .link((3, VALUE), (4, LEFT))
        .link((1, "delta"), (4, RIGHT))
        .link((4, RESULT), (2, VALUE))
}

#[test]
fn a_correct_graph_has_no_errors() {
    assert_eq!(scale_speed().errors(), []);
}

#[test]
fn branches_may_leave_outputs_unconnected() {
    let errors = GraphBuilder::new()
        .node(1, NodeKind::Entry)
        .node(2, NodeKind::Branch)
        .node(
            3,
            NodeKind::Literal {
                value: Literal::Bool(true),
            },
        )
        .node(4, NodeKind::Return)
        .link((1, EXECUTION_OUTPUT), (2, EXECUTION_INPUT))
        .link((3, VALUE), (2, CONDITION))
        .link((2, BRANCH_FALSE), (4, EXECUTION_INPUT))
        .errors();

    assert_eq!(errors, []);
}

#[test]
fn graphs_need_exactly_one_entry() {
    let none = GraphBuilder::new().node(1, NodeKind::Return).errors();
    let two = GraphBuilder::new()
        .node(1, NodeKind::Entry)
        .node(2, NodeKind::Entry)
        .errors();

    assert!(
        none.contains(&(None, BlueprintErrorKind::MissingEntry)),
        "{none:?}"
    );
    assert_eq!(two, [(None, BlueprintErrorKind::SeveralEntries)]);
}

#[test]
fn node_identifiers_are_unique() {
    let errors = GraphBuilder::new()
        .node(1, NodeKind::Entry)
        .node(1, get_speed())
        .errors();

    assert_eq!(errors, [(Some(1), BlueprintErrorKind::DuplicateNode)]);
}

#[test]
fn execution_inputs_take_one_wire_so_the_flow_is_a_tree() {
    let errors = GraphBuilder::new()
        .node(1, NodeKind::Entry)
        .node(2, NodeKind::Branch)
        .node(
            3,
            NodeKind::Literal {
                value: Literal::Bool(true),
            },
        )
        .node(4, NodeKind::Return)
        .link((1, EXECUTION_OUTPUT), (2, EXECUTION_INPUT))
        .link((3, VALUE), (2, CONDITION))
        .link((2, BRANCH_TRUE), (4, EXECUTION_INPUT))
        .link((2, BRANCH_FALSE), (4, EXECUTION_INPUT))
        .errors();

    assert_eq!(
        errors,
        [(
            Some(4),
            BlueprintErrorKind::PinUsedTwice(EXECUTION_INPUT.to_owned())
        )]
    );
}

#[test]
fn execution_outputs_take_one_wire() {
    let errors = GraphBuilder::new()
        .node(1, NodeKind::Entry)
        .node(2, NodeKind::Return)
        .node(3, NodeKind::Return)
        .link((1, EXECUTION_OUTPUT), (2, EXECUTION_INPUT))
        .link((1, EXECUTION_OUTPUT), (3, EXECUTION_INPUT))
        .errors();

    assert_eq!(
        errors,
        [(
            Some(1),
            BlueprintErrorKind::PinUsedTwice(EXECUTION_OUTPUT.to_owned())
        )]
    );
}

#[test]
fn data_outputs_feed_any_number_of_inputs() {
    let errors = GraphBuilder::new()
        .node(1, NodeKind::Entry)
        .node(2, set_speed())
        .node(3, operator(Operator::Add))
        .link((1, EXECUTION_OUTPUT), (2, EXECUTION_INPUT))
        .link((1, "delta"), (3, LEFT))
        .link((1, "delta"), (3, RIGHT))
        .link((3, RESULT), (2, VALUE))
        .errors();

    assert_eq!(errors, []);
}

#[test]
fn data_inputs_need_a_wire() {
    let errors = GraphBuilder::new()
        .node(1, NodeKind::Entry)
        .node(2, set_speed())
        .link((1, EXECUTION_OUTPUT), (2, EXECUTION_INPUT))
        .errors();

    assert_eq!(
        errors,
        [(
            Some(2),
            BlueprintErrorKind::UnconnectedInput(VALUE.to_owned())
        )]
    );
}

#[test]
fn nodes_the_execution_never_reaches_are_reported() {
    let errors = GraphBuilder::new()
        .node(1, NodeKind::Entry)
        .node(2, NodeKind::Return)
        .errors();

    assert_eq!(errors, [(Some(2), BlueprintErrorKind::Unreachable)]);
}

#[test]
fn links_must_go_from_an_output_to_an_input_of_the_same_kind() {
    let errors = scale_speed()
        .link((2, VALUE), (4, LEFT))
        .link((1, EXECUTION_OUTPUT), (4, RIGHT))
        .link((4, "power"), (2, VALUE))
        .link((9, RESULT), (2, VALUE))
        .errors();

    assert_eq!(
        errors,
        [
            (Some(2), BlueprintErrorKind::StartsAtInput(VALUE.to_owned())),
            (Some(4), BlueprintErrorKind::MixedPinKinds),
            (Some(4), BlueprintErrorKind::UnknownPin("power".to_owned())),
            (Some(9), BlueprintErrorKind::UnknownNode),
        ]
    );
}

#[test]
fn values_cannot_depend_on_themselves() {
    let errors = GraphBuilder::new()
        .node(1, NodeKind::Entry)
        .node(2, operator(Operator::Add))
        .node(3, operator(Operator::Negate))
        .link((1, "delta"), (2, LEFT))
        .link((3, RESULT), (2, RIGHT))
        .link((2, RESULT), (3, VALUE))
        .errors();

    assert_eq!(
        errors,
        [
            (Some(2), BlueprintErrorKind::DataCycle),
            (Some(3), BlueprintErrorKind::DataCycle),
        ]
    );
}

#[test]
fn names_must_be_valid_and_unique() {
    let call = Call {
        target: CallTarget::Class("Debug".to_owned()),
        method: "print me".to_owned(),
        arguments: vec!["text".to_owned(), "text".to_owned()],
        returns_value: false,
        pure: true,
    };
    let errors = GraphBuilder::new()
        .node(1, NodeKind::Entry)
        .node(2, NodeKind::Call(call))
        .link((1, "delta"), (2, "text"))
        .errors();

    assert!(
        errors.contains(&(
            Some(2),
            BlueprintErrorKind::InvalidName("print me".to_owned())
        )),
        "{errors:?}"
    );
    assert!(
        errors.contains(&(
            Some(2),
            BlueprintErrorKind::DuplicateParameter("text".to_owned())
        )),
        "{errors:?}"
    );
}

#[test]
fn class_rules_are_checked() {
    let mut blueprint = blueprint(vec![scale_speed().graph, scale_speed().graph]);
    blueprint.class = "2Player".to_owned();
    blueprint.variables.push(Variable {
        name: "speed".to_owned(),
        ty: None,
        default: None,
        private: false,
    });

    assert_eq!(
        blueprint_errors(&blueprint),
        [
            (None, BlueprintErrorKind::InvalidName("2Player".to_owned())),
            (
                None,
                BlueprintErrorKind::DuplicateVariable("speed".to_owned())
            ),
            (
                None,
                BlueprintErrorKind::VariableWithoutType("speed".to_owned())
            ),
            (
                None,
                BlueprintErrorKind::DuplicateGraph("update".to_owned())
            ),
        ]
    );
}

#[test]
fn errors_name_their_graph_and_node() {
    let error = BlueprintError {
        graph: Some("update".to_owned()),
        node: Some(NodeId(4)),
        kind: BlueprintErrorKind::Unreachable,
    };

    assert_eq!(
        error.to_string(),
        "graph `update`, node 4, the execution never reaches this node"
    );
}
