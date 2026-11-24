//! Translation of one graph: the method's declaration, its statements in
//! the order of the execution wires and the expressions of its data wires.

use std::collections::{HashMap, HashSet};

use super::writer::SourceWriter;
use super::{Origin, literal_code};
use crate::model::{Call, CallTarget, Graph, NodeId, NodeKind, PinRef};
use crate::pins::{
    BRANCH_FALSE, BRANCH_TRUE, CONDITION, ELEMENT, EXECUTION_INPUT, EXECUTION_OUTPUT, LEFT, LIST,
    LOOP_BODY, LOOP_DONE, RESULT, RIGHT, TARGET, VALUE,
};
use crate::validation::{BlueprintError, BlueprintErrorKind};

/// Translates one graph that follows every rule of [`validate`](crate::validate).
pub(super) struct GraphTranslator<'graph, 'out> {
    graph: &'graph Graph,
    writer: &'out mut SourceWriter,
    errors: &'out mut Vec<BlueprintError>,
    /// For each data input, the output it takes its value from.
    sources: HashMap<(NodeId, &'graph str), &'graph PinRef>,
    /// For each execution output, the node it leads to.
    next: HashMap<(NodeId, &'graph str), NodeId>,
    /// Calls that run in order and whose result some node uses.
    used_results: HashSet<NodeId>,
    /// Calls whose result already has a name, and loops whose element is
    /// known, by nesting level: a name given inside an `if` or a `for` is
    /// not known after it.
    named_results: Vec<HashSet<NodeId>>,
    /// Names of the method's parameters, which hide members of the class.
    parameters: HashSet<&'graph str>,
}

impl<'graph, 'out> GraphTranslator<'graph, 'out> {
    pub(super) fn new(
        graph: &'graph Graph,
        writer: &'out mut SourceWriter,
        errors: &'out mut Vec<BlueprintError>,
    ) -> Self {
        let mut sources = HashMap::new();
        let mut next = HashMap::new();
        let mut used_results = HashSet::new();
        for link in &graph.links {
            if link.to.pin == EXECUTION_INPUT {
                next.insert((link.from.node, link.from.pin.as_str()), link.to.node);
            } else {
                sources.insert((link.to.node, link.to.pin.as_str()), &link.from);
                if link.from.pin == RESULT {
                    used_results.insert(link.from.node);
                }
            }
        }
        Self {
            graph,
            writer,
            errors,
            sources,
            next,
            used_results,
            named_results: vec![HashSet::new()],
            parameters: graph.parameters.iter().map(|p| p.name.as_str()).collect(),
        }
    }

    pub(super) fn translate(mut self) {
        let start = self.writer.position();
        self.signature();
        self.writer
            .record(start, Origin::Graph(self.graph.name.clone()));
        self.writer.open();
        let entry = self
            .graph
            .nodes
            .iter()
            .find(|node| matches!(node.kind, NodeKind::Entry));
        if let Some(entry) = entry {
            self.statements(entry.id, EXECUTION_OUTPUT);
        }
        self.writer.close();
        self.writer.line("}");
    }

    /// `[private ][override ]func name(parameter: type, …)[ -> type] {`
    fn signature(&mut self) {
        let graph = self.graph;
        self.writer.indent();
        if graph.private {
            self.writer.write("private ");
        }
        if graph.is_override {
            self.writer.write("override ");
        }
        let parameters = graph
            .parameters
            .iter()
            .map(|parameter| format!("{}: {}", parameter.name, parameter.ty))
            .collect::<Vec<_>>()
            .join(", ");
        self.writer
            .write(&format!("func {}({parameters})", graph.name));
        if let Some(returns) = &graph.returns {
            self.writer.write(&format!(" -> {returns}"));
        }
        self.writer.write(" {");
        self.writer.newline();
    }

    /// The statements of the nodes that follow the execution output `pin`
    /// of `node`, one after the other.
    fn statements(&mut self, node: NodeId, pin: &str) {
        let mut current = self.next.get(&(node, pin)).copied();
        while let Some(id) = current {
            let start = self.writer.position();
            current = self.statement(id);
            self.record_node(start, id);
        }
    }

    /// Writes the statement of node `id`; returns the node that runs next.
    fn statement(&mut self, id: NodeId) -> Option<NodeId> {
        let node = self.graph.node(id)?;
        match &node.kind {
            NodeKind::SetVariable { variable, of_value } => {
                self.writer.indent();
                self.field(id, variable, *of_value);
                self.writer.write(" = ");
                self.input(id, VALUE, false);
                self.writer.newline();
                self.next.get(&(id, EXECUTION_OUTPUT)).copied()
            }
            NodeKind::Call(call) => {
                self.writer.indent();
                if self.used_results.contains(&id) {
                    self.writer
                        .write(&format!("const {} = ", result_name(call, id)));
                }
                self.call(id, call);
                self.writer.newline();
                if let Some(named) = self.named_results.last_mut() {
                    named.insert(id);
                }
                self.next.get(&(id, EXECUTION_OUTPUT)).copied()
            }
            NodeKind::Branch => {
                self.branch(id);
                None
            }
            NodeKind::ForEach => {
                self.for_each(id);
                self.next.get(&(id, LOOP_DONE)).copied()
            }
            NodeKind::Return => {
                self.writer.indent();
                self.writer.write("return");
                if self.graph.returns.is_some() {
                    self.writer.write(" ");
                    self.input(id, VALUE, false);
                }
                self.writer.newline();
                None
            }
            NodeKind::Entry
            | NodeKind::Literal { .. }
            | NodeKind::GetVariable { .. }
            | NodeKind::Operator { .. } => None,
        }
    }

    /// `if condition { … } else { … }`; a branch with only its `false` path
    /// is written as `if !condition { … }`.
    fn branch(&mut self, id: NodeId) {
        let has_true = self.next.contains_key(&(id, BRANCH_TRUE));
        let has_false = self.next.contains_key(&(id, BRANCH_FALSE));
        self.writer.indent();
        self.writer.write("if ");
        if has_true || !has_false {
            self.input(id, CONDITION, false);
            self.writer.write(" {");
            self.writer.newline();
            self.block(id, BRANCH_TRUE);
            if has_false {
                self.writer.line("} else {");
                self.block(id, BRANCH_FALSE);
            }
        } else {
            self.writer.write("!");
            self.input(id, CONDITION, true);
            self.writer.write(" {");
            self.writer.newline();
            self.block(id, BRANCH_FALSE);
        }
        self.writer.line("}");
    }

    /// `for element_N in list { … }`: its body, where its element is
    /// known.
    fn for_each(&mut self, id: NodeId) {
        self.writer.indent();
        self.writer.write(&format!("for {} in ", element_name(id)));
        self.input(id, LIST, false);
        self.writer.write(" {");
        self.writer.newline();
        self.writer.open();
        self.named_results.push(HashSet::from([id]));
        self.statements(id, LOOP_BODY);
        self.named_results.pop();
        self.writer.close();
        self.writer.line("}");
    }

    /// The statements after `pin` of `node`, one level deeper.
    fn block(&mut self, node: NodeId, pin: &str) {
        self.writer.open();
        self.named_results.push(HashSet::new());
        self.statements(node, pin);
        self.named_results.pop();
        self.writer.close();
    }

    /// `target.method(arguments)`
    fn call(&mut self, id: NodeId, call: &Call) {
        match &call.target {
            CallTarget::OwnObject => self.member(&call.method),
            CallTarget::Value => {
                self.input(id, TARGET, true);
                self.writer.write(&format!(".{}", call.method));
            }
            CallTarget::Class(class) => self.writer.write(&format!("{class}.{}", call.method)),
        }
        self.writer.write("(");
        for (index, argument) in call.arguments.iter().enumerate() {
            if index > 0 {
                self.writer.write(", ");
            }
            self.input(id, argument, false);
        }
        self.writer.write(")");
    }

    /// The field `name` of the object given to the `target` pin of node
    /// `id` if `of_value`, or else of the blueprint.
    fn field(&mut self, id: NodeId, name: &str, of_value: bool) {
        if of_value {
            self.input(id, TARGET, true);
            self.writer.write(&format!(".{name}"));
        } else {
            self.member(name);
        }
    }

    /// A field or method of the object: `self.` only if a parameter hides it.
    fn member(&mut self, name: &str) {
        if self.parameters.contains(name) {
            self.writer.write("self.");
        }
        self.writer.write(name);
    }

    /// The value of the data input `pin` of `node`. Operators are put
    /// between parentheses if `inside_expression`.
    fn input(&mut self, node: NodeId, pin: &str, inside_expression: bool) {
        if let Some(source) = self.sources.get(&(node, pin)).copied() {
            self.value(source, inside_expression);
        }
    }

    /// The value of the output `source`.
    fn value(&mut self, source: &PinRef, inside_expression: bool) {
        let Some(node) = self.graph.node(source.node) else {
            return;
        };
        let id = node.id;
        let start = self.writer.position();
        match &node.kind {
            NodeKind::Entry => self.writer.write(&source.pin),
            NodeKind::Literal { value } => match literal_code(value) {
                Some(code) => self.writer.write(&code),
                None => self.error(id, BlueprintErrorKind::UnwritableNumber),
            },
            NodeKind::GetVariable { variable, of_value } => self.field(id, variable, *of_value),
            NodeKind::Operator { operator } => {
                if inside_expression {
                    self.writer.write("(");
                }
                if operator.is_unary() {
                    self.writer.write(operator.symbol());
                    self.input(id, VALUE, true);
                } else {
                    self.input(id, LEFT, true);
                    self.writer.write(&format!(" {} ", operator.symbol()));
                    self.input(id, RIGHT, true);
                }
                if inside_expression {
                    self.writer.write(")");
                }
            }
            NodeKind::Call(call) if call.pure => self.call(id, call),
            NodeKind::Call(call) => {
                if !self.is_named(id) {
                    self.error(id, BlueprintErrorKind::ValueNotReady);
                }
                self.writer.write(&result_name(call, id));
            }
            NodeKind::ForEach if source.pin == ELEMENT => {
                if !self.is_named(id) {
                    self.error(id, BlueprintErrorKind::ValueNotReady);
                }
                self.writer.write(&element_name(id));
            }
            NodeKind::SetVariable { .. }
            | NodeKind::Branch
            | NodeKind::ForEach
            | NodeKind::Return => {}
        }
        self.record_node(start, id);
    }

    /// Whether the value of node `id` has a name here: a call already run,
    /// or a loop this code is inside of.
    fn is_named(&self, id: NodeId) -> bool {
        self.named_results.iter().any(|names| names.contains(&id))
    }

    fn record_node(&mut self, start: usize, node: NodeId) {
        let origin = Origin::Node {
            graph: self.graph.name.clone(),
            node,
        };
        self.writer.record(start, origin);
    }

    fn error(&mut self, node: NodeId, kind: BlueprintErrorKind) {
        self.errors.push(BlueprintError {
            graph: Some(self.graph.name.clone()),
            node: Some(node),
            kind,
        });
    }
}

/// The name of the element of a loop: `element_4`.
fn element_name(id: NodeId) -> String {
    format!("{ELEMENT}_{}", id.0)
}

/// The name of the constant that keeps the result of a call that runs in
/// order: its method and its node, `is_down_4`.
fn result_name(call: &Call, id: NodeId) -> String {
    format!("{}_{}", call.method, id.0)
}
