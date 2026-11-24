use star_script::{GameSettings, InputAction, check_source, check_source_with};

use super::*;
use crate::model::{
    Call, CallTarget, Graph, InspectorOptions, Link, Node, NodeKind, Operator, Parameter, PinRef,
};
use crate::pins::{
    BRANCH_FALSE, BRANCH_TRUE, CONDITION, EXECUTION_INPUT, EXECUTION_OUTPUT, LEFT, RESULT, RIGHT,
    TARGET, VALUE,
};

const PLAYER: &str = include_str!("../../examples/player.blueprint");

/// A blueprint `Ship extends Node` with one graph.
fn ship(graph: Graph) -> Blueprint {
    Blueprint {
        class: "Ship".to_owned(),
        extends: Some("Node".to_owned()),
        base_arguments: vec![Literal::String("Ship".to_owned())],
        variables: vec![Variable {
            name: "speed".to_owned(),
            ty: None,
            default: Some(Literal::Float(2.0)),
            private: false,
            export: false,
            inspector: InspectorOptions::default(),
        }],
        graphs: vec![graph],
    }
}

/// A graph called `name`, built node by node.
struct GraphBuilder(Graph);

impl GraphBuilder {
    fn new(name: &str, parameters: &[(&str, &str)], returns: Option<&str>) -> Self {
        Self(Graph {
            name: name.to_owned(),
            parameters: parameters
                .iter()
                .map(|(name, ty)| Parameter {
                    name: (*name).to_owned(),
                    ty: (*ty).to_owned(),
                })
                .collect(),
            returns: returns.map(str::to_owned),
            is_override: false,
            private: false,
            nodes: vec![Node {
                id: NodeId(1),
                position: [0.0, 0.0],
                kind: NodeKind::Entry,
            }],
            links: Vec::new(),
        })
    }

    fn overriding(mut self) -> Self {
        self.0.is_override = true;
        self
    }

    fn node(mut self, id: u32, kind: NodeKind) -> Self {
        self.0.nodes.push(Node {
            id: NodeId(id),
            position: [0.0, 0.0],
            kind,
        });
        self
    }

    fn link(mut self, from: (u32, &str), to: (u32, &str)) -> Self {
        self.0.links.push(Link {
            from: PinRef::new(NodeId(from.0), from.1),
            to: PinRef::new(NodeId(to.0), to.1),
        });
        self
    }

    fn build(self) -> Graph {
        self.0
    }
}

fn call(target: CallTarget, method: &str, arguments: &[&str], pure: bool) -> NodeKind {
    NodeKind::Call(Call {
        target,
        method: method.to_owned(),
        arguments: arguments.iter().map(|name| (*name).to_owned()).collect(),
        returns_value: true,
        pure,
    })
}

fn literal(value: Literal) -> NodeKind {
    NodeKind::Literal { value }
}

fn operator(operator: Operator) -> NodeKind {
    NodeKind::Operator { operator }
}

fn variable(name: &str, set: bool) -> NodeKind {
    let variable = name.to_owned();
    if set {
        NodeKind::SetVariable {
            variable,
            of_value: false,
        }
    } else {
        NodeKind::GetVariable {
            variable,
            of_value: false,
        }
    }
}

/// The translation, checked by the StarScript compiler in a game with the
/// input action `fast`.
fn checked_code(blueprint: &Blueprint) -> String {
    let translation = translate(blueprint).expect("translatable");
    let source = translation.source().to_owned();
    let settings = GameSettings {
        actions: vec![InputAction {
            name: "fast".to_owned(),
            bindings: vec!["ShiftLeft".to_owned()],
        }],
    };
    if let Err(errors) = check_source_with(&source, &settings) {
        panic!("{source}\n{errors:?}");
    }
    source
}

/// The body of the method, without the header, class and constructor.
fn method_lines(code: &str) -> Vec<&str> {
    let start = code.find("func ").expect("a method");
    code[start..]
        .lines()
        .skip(1)
        .take_while(|line| *line != "    }")
        .map(str::trim)
        .collect()
}

#[test]
fn the_example_of_the_documentation_translates_to_readable_code() {
    let blueprint = Blueprint::from_text(PLAYER).expect("valid file");

    assert_eq!(
        checked_code(&blueprint),
        "// Translated from a blueprint: change the blueprint, not this code.\n\
         class Player extends Sprite {\n    \
             var speed = 2.0\n\
         \n    \
             init() {\n        \
                 super.init(\"Player\", \"sprites/player.png\")\n    \
             }\n\
         \n    \
             override func update(delta: float) {\n        \
                 speed = speed * delta\n    \
             }\n\
         }\n"
    );
}

#[test]
fn branches_become_if_and_else() {
    let graph = GraphBuilder::new("update", &[("delta", "float")], None)
        .overriding()
        .node(2, NodeKind::Branch)
        .node(
            3,
            call(
                CallTarget::Class("Input".to_owned()),
                "is_down",
                &["action"],
                true,
            ),
        )
        .node(4, literal(Literal::String("fast".to_owned())))
        .node(5, variable("speed", true))
        .node(6, literal(Literal::Float(4.0)))
        .node(7, variable("speed", true))
        .node(8, literal(Literal::Float(1.0)))
        .link((1, EXECUTION_OUTPUT), (2, EXECUTION_INPUT))
        .link((4, VALUE), (3, "action"))
        .link((3, RESULT), (2, CONDITION))
        .link((2, BRANCH_TRUE), (5, EXECUTION_INPUT))
        .link((6, VALUE), (5, VALUE))
        .link((2, BRANCH_FALSE), (7, EXECUTION_INPUT))
        .link((8, VALUE), (7, VALUE))
        .build();

    let code = checked_code(&ship(graph));

    assert_eq!(
        method_lines(&code),
        [
            "if Input.is_down(\"fast\") {",
            "speed = 4.0",
            "} else {",
            "speed = 1.0",
            "}",
        ]
    );
}

#[test]
fn a_branch_with_only_its_false_path_negates_the_condition() {
    let graph = GraphBuilder::new("check", &[("level", "int")], Some("int"))
        .node(2, NodeKind::Branch)
        .node(3, operator(Operator::Less))
        .node(4, literal(Literal::Int(3)))
        .node(5, NodeKind::Return)
        .link((1, EXECUTION_OUTPUT), (2, EXECUTION_INPUT))
        .link((1, "level"), (3, LEFT))
        .link((4, VALUE), (3, RIGHT))
        .link((3, RESULT), (2, CONDITION))
        .link((2, BRANCH_FALSE), (5, EXECUTION_INPUT))
        .link((1, "level"), (5, VALUE))
        .build();

    let code = translate(&ship(graph)).expect("translatable");

    assert_eq!(
        method_lines(code.source()),
        ["if !(level < 3) {", "return level", "}"]
    );
}

#[test]
fn operators_inside_operators_get_parentheses() {
    let graph = GraphBuilder::new("grow", &[("delta", "float")], None)
        .node(2, variable("speed", true))
        .node(3, operator(Operator::Multiply))
        .node(4, operator(Operator::Add))
        .node(5, variable("speed", false))
        .node(6, operator(Operator::Negate))
        .link((1, EXECUTION_OUTPUT), (2, EXECUTION_INPUT))
        .link((5, VALUE), (4, LEFT))
        .link((1, "delta"), (4, RIGHT))
        .link((4, RESULT), (3, LEFT))
        .link((1, "delta"), (6, VALUE))
        .link((6, RESULT), (3, RIGHT))
        .link((3, RESULT), (2, VALUE))
        .build();

    let code = checked_code(&ship(graph));

    assert_eq!(method_lines(&code), ["speed = (speed + delta) * (-delta)"]);
}

#[test]
fn results_of_calls_that_run_in_order_are_kept_in_constants() {
    let graph = GraphBuilder::new("twice", &[], Some("Node"))
        .node(2, call(CallTarget::OwnObject, "make", &[], false))
        .node(3, NodeKind::Return)
        .link((1, EXECUTION_OUTPUT), (2, EXECUTION_INPUT))
        .link((2, EXECUTION_OUTPUT), (3, EXECUTION_INPUT))
        .link((2, RESULT), (3, VALUE))
        .build();
    let mut blueprint = ship(graph);
    blueprint.graphs.push(
        GraphBuilder::new("make", &[], Some("Node"))
            .node(2, NodeKind::Return)
            .node(
                3,
                call(CallTarget::Class("Node".to_owned()), "find", &[], true),
            )
            .link((1, EXECUTION_OUTPUT), (2, EXECUTION_INPUT))
            .link((3, RESULT), (2, VALUE))
            .build(),
    );

    let code = translate(&blueprint).expect("translatable");

    assert_eq!(
        method_lines(code.source()),
        ["const make_2 = make()", "return make_2"]
    );
}

#[test]
fn results_used_outside_the_path_where_they_run_are_rejected() {
    let graph = GraphBuilder::new("pick", &[("fast", "bool")], Some("int"))
        .node(2, NodeKind::Branch)
        .node(3, call(CallTarget::OwnObject, "roll", &[], false))
        .node(4, NodeKind::Return)
        .link((1, EXECUTION_OUTPUT), (2, EXECUTION_INPUT))
        .link((1, "fast"), (2, CONDITION))
        .link((2, BRANCH_TRUE), (3, EXECUTION_INPUT))
        .link((2, BRANCH_FALSE), (4, EXECUTION_INPUT))
        .link((3, RESULT), (4, VALUE))
        .build();

    let errors = translate(&ship(graph)).expect_err("the result is not ready");

    assert_eq!(
        errors,
        [BlueprintError {
            graph: Some("pick".to_owned()),
            node: Some(NodeId(3)),
            kind: BlueprintErrorKind::ValueNotReady,
        }]
    );
}

#[test]
fn members_hidden_by_parameters_are_written_with_self() {
    let graph = GraphBuilder::new("set_speed", &[("speed", "float")], None)
        .node(2, variable("speed", true))
        .link((1, EXECUTION_OUTPUT), (2, EXECUTION_INPUT))
        .link((1, "speed"), (2, VALUE))
        .build();

    let code = checked_code(&ship(graph));

    assert_eq!(method_lines(&code), ["self.speed = speed"]);
}

#[test]
fn calls_on_values_write_their_target() {
    let graph = GraphBuilder::new("hit", &[("other", "Node")], None)
        .node(2, call(CallTarget::Value, "add_child", &["child"], false))
        .node(
            3,
            call(CallTarget::Class("Node".to_owned()), "new_rock", &[], true),
        )
        .link((1, EXECUTION_OUTPUT), (2, EXECUTION_INPUT))
        .link((1, "other"), (2, TARGET))
        .link((3, RESULT), (2, "child"))
        .build();

    let code = translate(&ship(graph)).expect("translatable");

    assert_eq!(
        method_lines(code.source()),
        ["other.add_child(Node.new_rock())"]
    );
}

#[test]
fn compiler_errors_are_placed_on_their_node() {
    let graph = GraphBuilder::new("update", &[("delta", "float")], None)
        .overriding()
        .node(2, variable("speed", true))
        .node(3, literal(Literal::String("fast".to_owned())))
        .link((1, EXECUTION_OUTPUT), (2, EXECUTION_INPUT))
        .link((3, VALUE), (2, VALUE))
        .build();
    let translation = translate(&ship(graph)).expect("translatable");

    let errors = check_source(translation.source()).expect_err("a text is not a float");

    assert_eq!(
        translation.origin(errors[0].span().start()),
        Some(&Origin::Node {
            graph: "update".to_owned(),
            node: NodeId(3),
        })
    );
}

#[test]
fn every_piece_of_code_has_an_origin() {
    let translation =
        translate(&Blueprint::from_text(PLAYER).expect("valid file")).expect("translatable");
    let source = translation.source();
    let origin_of = |text: &str| {
        let offset = source.find(text).expect("in the code");
        translation.origin(offset).cloned()
    };

    assert_eq!(origin_of("class Player"), Some(Origin::Class));
    assert_eq!(
        origin_of("var speed"),
        Some(Origin::Variable("speed".to_owned()))
    );
    assert_eq!(
        origin_of("override func"),
        Some(Origin::Graph("update".to_owned()))
    );
    assert_eq!(
        origin_of("delta\n"),
        Some(Origin::Node {
            graph: "update".to_owned(),
            node: NodeId(1),
        })
    );
    assert_eq!(translation.origin(0), None);
}

#[test]
fn variables_of_a_value_read_and_set_its_fields() {
    let field = |set: bool| {
        let variable = if set { "x" } else { "y" }.to_owned();
        if set {
            NodeKind::SetVariable {
                variable,
                of_value: true,
            }
        } else {
            NodeKind::GetVariable {
                variable,
                of_value: true,
            }
        }
    };
    let graph = GraphBuilder::new("follow", &[("other", "Node")], None)
        .node(2, field(true))
        .node(3, field(false))
        .link((1, EXECUTION_OUTPUT), (2, EXECUTION_INPUT))
        .link((1, "other"), (2, TARGET))
        .link((1, "other"), (3, TARGET))
        .link((3, VALUE), (2, VALUE))
        .build();

    let code = checked_code(&ship(graph));

    assert_eq!(method_lines(&code), ["other.x = other.y"]);
}

#[test]
fn exported_variables_are_written_with_export() {
    let mut blueprint = ship(GraphBuilder::new("start", &[], None).overriding().build());
    blueprint.variables[0].export = true;

    let code = checked_code(&blueprint);

    assert!(code.contains("    export var speed = 2.0\n"), "{code}");
}

#[test]
fn inspector_options_are_written_as_decorators() {
    let mut blueprint = ship(GraphBuilder::new("start", &[], None).overriding().build());
    let speed = &mut blueprint.variables[0];
    speed.export = true;
    speed.inspector = InspectorOptions {
        range: vec![0.0, 10.0, 0.5],
        tooltip: Some("Units \"a\" second".to_owned()),
        header: Some("Motion".to_owned()),
        multiline: false,
    };

    let code = checked_code(&blueprint);

    assert!(
        code.contains(
            "    @header(\"Motion\")\n    @range(0.0, 10.0, 0.5)\n    \
             @tooltip(\"Units \\\"a\\\" second\")\n    export var speed = 2.0\n"
        ),
        "{code}"
    );
}
