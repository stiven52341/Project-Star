//! The rules a blueprint must follow to be translated to StarScript.
//!
//! Types are not checked here: the StarScript compiler checks them in the
//! translated code, and its errors are placed back on the nodes.

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;

use crate::model::{Blueprint, CallTarget, Graph, Link, Node, NodeId, NodeKind};
use crate::pins::{Direction, EXECUTION_INPUT, PinKind};

/// A rule of blueprints that is broken, and where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlueprintError {
    /// The graph it is in, if it is inside one.
    pub graph: Option<String>,
    /// The node it is about, if it is about one.
    pub node: Option<NodeId>,
    /// What is wrong.
    pub kind: BlueprintErrorKind,
}

/// Every rule of blueprints that can be broken.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlueprintErrorKind {
    /// A class, variable, graph, parameter, method or argument name that
    /// StarScript would not accept.
    InvalidName(String),
    /// Two variables with the same name.
    DuplicateVariable(String),
    /// A variable without a type or a starting value to take it from.
    VariableWithoutType(String),
    /// Two graphs with the same name.
    DuplicateGraph(String),
    /// Two pins of a node with the same name, such as two parameters of a
    /// graph or an argument of a call called like another of its pins.
    DuplicatePin(String),
    /// Two nodes of a graph with the same identifier.
    DuplicateNode,
    /// A graph without an entry node.
    MissingEntry,
    /// A graph with more than one entry node.
    SeveralEntries,
    /// A link to a node that is not in the graph.
    UnknownNode,
    /// A link to a pin the node does not have.
    UnknownPin(String),
    /// A link that starts at an input pin.
    StartsAtInput(String),
    /// A link that ends at an output pin.
    EndsAtOutput(String),
    /// A link between an execution pin and a data pin.
    MixedPinKinds,
    /// A pin with more links than it takes: an execution output or any input.
    PinUsedTwice(String),
    /// A data input without a link.
    UnconnectedInput(String),
    /// A node that runs in order but the execution never reaches.
    Unreachable,
    /// A value that depends on itself.
    DataCycle,
    /// The result of a call that runs in order, used where that call has
    /// not run: before it, or in another path of a branch.
    ValueNotReady,
    /// A number that is not finite, which StarScript cannot write.
    UnwritableNumber,
}

impl fmt::Display for BlueprintErrorKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidName(name) => write!(formatter, "`{name}` is not a valid name"),
            Self::DuplicateVariable(name) => {
                write!(formatter, "the variable `{name}` is declared twice")
            }
            Self::VariableWithoutType(name) => write!(
                formatter,
                "the variable `{name}` needs a type or a starting value"
            ),
            Self::DuplicateGraph(name) => write!(formatter, "there are two graphs called `{name}`"),
            Self::DuplicatePin(name) => write!(formatter, "two pins are called `{name}`"),
            Self::DuplicateNode => formatter.write_str("two nodes have this identifier"),
            Self::MissingEntry => formatter.write_str("the graph has no entry node"),
            Self::SeveralEntries => formatter.write_str("the graph has more than one entry node"),
            Self::UnknownNode => formatter.write_str("a link goes to a node that does not exist"),
            Self::UnknownPin(pin) => write!(formatter, "the node has no pin `{pin}`"),
            Self::StartsAtInput(pin) => write!(formatter, "a link starts at the input `{pin}`"),
            Self::EndsAtOutput(pin) => write!(formatter, "a link ends at the output `{pin}`"),
            Self::MixedPinKinds => {
                formatter.write_str("a link joins an execution pin and a value pin")
            }
            Self::PinUsedTwice(pin) => write!(formatter, "the pin `{pin}` has more than one link"),
            Self::UnconnectedInput(pin) => write!(formatter, "the input `{pin}` has no link"),
            Self::Unreachable => formatter.write_str("the execution never reaches this node"),
            Self::DataCycle => formatter.write_str("this value depends on itself"),
            Self::ValueNotReady => {
                formatter.write_str("its result is used where the node has not run yet")
            }
            Self::UnwritableNumber => formatter.write_str("the number is not finite"),
        }
    }
}

impl fmt::Display for BlueprintError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(graph) = &self.graph {
            write!(formatter, "graph `{graph}`, ")?;
        }
        if let Some(NodeId(node)) = self.node {
            write!(formatter, "node {node}, ")?;
        }
        self.kind.fmt(formatter)
    }
}

impl Error for BlueprintError {}

/// Checks every rule of blueprints.
///
/// # Errors
///
/// Returns every broken rule: of the class first, then of each graph.
pub fn validate(blueprint: &Blueprint) -> Result<(), Vec<BlueprintError>> {
    let mut errors = Vec::new();
    check_class(blueprint, &mut errors);
    for graph in &blueprint.graphs {
        GraphCheck::new(graph, &mut errors).run();
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn class_error(kind: BlueprintErrorKind) -> BlueprintError {
    BlueprintError {
        graph: None,
        node: None,
        kind,
    }
}

fn check_class(blueprint: &Blueprint, errors: &mut Vec<BlueprintError>) {
    let names = std::iter::once(&blueprint.class).chain(&blueprint.extends);
    for name in names.filter(|name| !is_valid_name(name)) {
        errors.push(class_error(BlueprintErrorKind::InvalidName(name.clone())));
    }
    let mut variables = HashSet::new();
    for variable in &blueprint.variables {
        let name = &variable.name;
        if !is_valid_name(name) {
            errors.push(class_error(BlueprintErrorKind::InvalidName(name.clone())));
        }
        if !variables.insert(name) {
            errors.push(class_error(BlueprintErrorKind::DuplicateVariable(
                name.clone(),
            )));
        }
        if variable.ty.is_none() && variable.default.is_none() {
            errors.push(class_error(BlueprintErrorKind::VariableWithoutType(
                name.clone(),
            )));
        }
    }
    let mut graphs = HashSet::new();
    for graph in &blueprint.graphs {
        if !graphs.insert(&graph.name) {
            errors.push(class_error(BlueprintErrorKind::DuplicateGraph(
                graph.name.clone(),
            )));
        }
    }
}

/// The checks of one graph.
struct GraphCheck<'check> {
    graph: &'check Graph,
    errors: &'check mut Vec<BlueprintError>,
}

impl<'check> GraphCheck<'check> {
    const fn new(graph: &'check Graph, errors: &'check mut Vec<BlueprintError>) -> Self {
        Self { graph, errors }
    }

    fn error(&mut self, node: Option<NodeId>, kind: BlueprintErrorKind) {
        self.errors.push(BlueprintError {
            graph: Some(self.graph.name.clone()),
            node,
            kind,
        });
    }

    fn run(mut self) {
        self.check_signature();
        self.check_nodes();
        let valid_links = self.check_links();
        self.check_pin_use(&valid_links);
        self.check_reachable(&valid_links);
        self.check_data_cycles(&valid_links);
    }

    fn check_signature(&mut self) {
        if !is_valid_name(&self.graph.name) {
            self.error(
                None,
                BlueprintErrorKind::InvalidName(self.graph.name.clone()),
            );
        }
        let names: Vec<&String> = self.graph.parameters.iter().map(|p| &p.name).collect();
        self.check_names(None, &names);
    }

    /// `names` are valid StarScript names.
    fn check_names(&mut self, node: Option<NodeId>, names: &[&String]) {
        for name in names.iter().filter(|name| !is_valid_name(name)) {
            self.error(node, BlueprintErrorKind::InvalidName((*name).clone()));
        }
    }

    /// The pins of `node` have different names, so a link names only one.
    fn check_pin_names(&mut self, node: &Node) {
        let mut seen = HashSet::new();
        for pin in node.pins(self.graph) {
            if seen.contains(&pin.name) {
                self.error(Some(node.id), BlueprintErrorKind::DuplicatePin(pin.name));
            } else {
                seen.insert(pin.name);
            }
        }
    }

    fn check_nodes(&mut self) {
        let mut ids = HashSet::new();
        let mut entries = 0_usize;
        for node in &self.graph.nodes {
            if !ids.insert(node.id) {
                self.error(Some(node.id), BlueprintErrorKind::DuplicateNode);
            }
            self.check_pin_names(node);
            match &node.kind {
                NodeKind::Entry => entries += 1,
                NodeKind::GetVariable { variable, .. } | NodeKind::SetVariable { variable, .. } => {
                    self.check_names(Some(node.id), &[variable]);
                }
                NodeKind::Call(call) => {
                    let mut names = vec![&call.method];
                    if let CallTarget::Class(class) = &call.target {
                        names.push(class);
                    }
                    self.check_names(Some(node.id), &names);
                    let arguments: Vec<&String> = call.arguments.iter().collect();
                    self.check_names(Some(node.id), &arguments);
                }
                NodeKind::Literal { .. }
                | NodeKind::Operator { .. }
                | NodeKind::Branch
                | NodeKind::ForEach
                | NodeKind::Return => {}
            }
        }
        match entries {
            0 => self.error(None, BlueprintErrorKind::MissingEntry),
            1 => {}
            _ => self.error(None, BlueprintErrorKind::SeveralEntries),
        }
    }

    /// Reports the links that join pins wrongly; returns the others, with
    /// what their pins carry.
    fn check_links(&mut self) -> Vec<(&'check Link, PinKind)> {
        let graph = self.graph;
        let mut valid = Vec::new();
        for link in &graph.links {
            let (Some(from), Some(to)) = (graph.node(link.from.node), graph.node(link.to.node))
            else {
                let missing = if graph.node(link.from.node).is_none() {
                    link.from.node
                } else {
                    link.to.node
                };
                self.error(Some(missing), BlueprintErrorKind::UnknownNode);
                continue;
            };
            let Some(start) = from.pin(graph, &link.from.pin) else {
                self.error(
                    Some(from.id),
                    BlueprintErrorKind::UnknownPin(link.from.pin.clone()),
                );
                continue;
            };
            let Some(end) = to.pin(graph, &link.to.pin) else {
                self.error(
                    Some(to.id),
                    BlueprintErrorKind::UnknownPin(link.to.pin.clone()),
                );
                continue;
            };
            if start.direction != Direction::Output {
                self.error(Some(from.id), BlueprintErrorKind::StartsAtInput(start.name));
            } else if end.direction != Direction::Input {
                self.error(Some(to.id), BlueprintErrorKind::EndsAtOutput(end.name));
            } else if start.kind != end.kind {
                self.error(Some(to.id), BlueprintErrorKind::MixedPinKinds);
            } else {
                valid.push((link, start.kind));
            }
        }
        valid
    }

    /// Every input and every execution output has at most one link, and
    /// every data input has one.
    fn check_pin_use(&mut self, links: &[(&Link, PinKind)]) {
        let mut uses: HashMap<(NodeId, &str), usize> = HashMap::new();
        for (link, kind) in links {
            *uses.entry((link.to.node, &link.to.pin)).or_default() += 1;
            if *kind == PinKind::Execution {
                *uses.entry((link.from.node, &link.from.pin)).or_default() += 1;
            }
        }
        let graph = self.graph;
        for node in &graph.nodes {
            for pin in node.pins(graph) {
                let count = uses
                    .get(&(node.id, pin.name.as_str()))
                    .copied()
                    .unwrap_or(0);
                if count > 1 {
                    self.error(Some(node.id), BlueprintErrorKind::PinUsedTwice(pin.name));
                } else if count == 0
                    && pin.kind == PinKind::Data
                    && pin.direction == Direction::Input
                {
                    self.error(
                        Some(node.id),
                        BlueprintErrorKind::UnconnectedInput(pin.name),
                    );
                }
            }
        }
    }

    /// Every node that runs in order is reached by the execution wires.
    fn check_reachable(&mut self, links: &[(&Link, PinKind)]) {
        let graph = self.graph;
        let mut next: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        for (link, kind) in links {
            if *kind == PinKind::Execution && link.to.pin == EXECUTION_INPUT {
                next.entry(link.from.node).or_default().push(link.to.node);
            }
        }
        let mut reached = HashSet::new();
        let mut pending: Vec<NodeId> = graph
            .nodes
            .iter()
            .filter(|node| matches!(node.kind, NodeKind::Entry))
            .map(|node| node.id)
            .collect();
        while let Some(id) = pending.pop() {
            if reached.insert(id) {
                pending.extend(next.get(&id).into_iter().flatten().copied());
            }
        }
        for node in &graph.nodes {
            if node.is_executed()
                && !reached.contains(&node.id)
                && !matches!(node.kind, NodeKind::Entry)
            {
                self.error(Some(node.id), BlueprintErrorKind::Unreachable);
            }
        }
    }

    /// No value depends on itself through data wires.
    fn check_data_cycles(&mut self, links: &[(&Link, PinKind)]) {
        let mut inputs: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        for (link, kind) in links {
            if *kind == PinKind::Data {
                inputs.entry(link.to.node).or_default().push(link.from.node);
            }
        }
        let mut finished = HashSet::new();
        let mut in_cycle = Vec::new();
        for node in &self.graph.nodes {
            visit_inputs(
                node.id,
                &inputs,
                &mut Vec::new(),
                &mut finished,
                &mut in_cycle,
            );
        }
        in_cycle.sort_unstable();
        in_cycle.dedup();
        for node in in_cycle {
            self.error(Some(node), BlueprintErrorKind::DataCycle);
        }
    }
}

/// Depth-first walk through the nodes `node` takes values from; a node met
/// again while its own inputs are being walked closes a cycle.
fn visit_inputs(
    node: NodeId,
    inputs: &HashMap<NodeId, Vec<NodeId>>,
    path: &mut Vec<NodeId>,
    finished: &mut HashSet<NodeId>,
    in_cycle: &mut Vec<NodeId>,
) {
    if finished.contains(&node) {
        return;
    }
    if let Some(start) = path.iter().position(|id| *id == node) {
        in_cycle.extend_from_slice(&path[start..]);
        return;
    }
    path.push(node);
    for input in inputs.get(&node).into_iter().flatten() {
        visit_inputs(*input, inputs, path, finished, in_cycle);
    }
    path.pop();
    finished.insert(node);
}

/// Whether StarScript accepts `name` as a name: an ASCII letter or `_`,
/// then letters, digits or `_`. Keywords are left to the compiler.
/// Whether StarScript accepts `name` for a class, variable, method,
/// parameter or argument: a letter or `_`, then letters, digits or `_`.
#[must_use]
pub fn is_valid_name(name: &str) -> bool {
    let mut characters = name.chars();
    characters
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && characters.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

#[cfg(test)]
mod tests;
