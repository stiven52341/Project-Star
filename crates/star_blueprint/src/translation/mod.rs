//! Translation of blueprints to StarScript.
//!
//! The code is written to be read: names as the blueprint gives them,
//! `self.` only where a parameter hides a member, and parentheses only
//! around operators inside other expressions. Each piece of it remembers
//! what it was written for, so a mistake the StarScript compiler finds in it
//! is placed back on its node ([`Translation::origin`]).

mod graph;
mod literals;
mod writer;

use std::fmt;

use crate::model::{Blueprint, Literal, NodeId, Variable};
use crate::validation::{BlueprintError, BlueprintErrorKind, validate};

use self::graph::GraphTranslator;
use self::literals::literal_code;
use self::writer::SourceWriter;

/// First line of every translation: the code belongs to the game's author,
/// so it carries no license notice, only where it comes from.
const HEADER: &str = "// Translated from a blueprint: change the blueprint, not this code.";

/// What a piece of a translation was written for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    /// The class itself: its declaration and braces.
    Class,
    /// The declaration of a variable.
    Variable(String),
    /// The declaration of a graph's method.
    Graph(String),
    /// A node of a graph.
    Node {
        /// The graph's name.
        graph: String,
        /// The node.
        node: NodeId,
    },
}

impl fmt::Display for Origin {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Class => formatter.write_str("class"),
            Self::Variable(name) => write!(formatter, "variable `{name}`"),
            Self::Graph(name) => write!(formatter, "graph `{name}`"),
            Self::Node {
                graph,
                node: NodeId(node),
            } => write!(formatter, "graph `{graph}`, node {node}"),
        }
    }
}

/// A piece of a translation and what it was written for.
#[derive(Debug, Clone, PartialEq, Eq)]
struct OriginRange {
    start: usize,
    end: usize,
    origin: Origin,
}

/// A blueprint translated to StarScript.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Translation {
    source: String,
    origins: Vec<OriginRange>,
}

impl Translation {
    /// The StarScript code.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// What the code at byte `offset` of [`Translation::source`] was written
    /// for: the smallest piece that holds it, such as the node of an
    /// expression rather than the node of the statement around it.
    #[must_use]
    pub fn origin(&self, offset: usize) -> Option<&Origin> {
        self.origins
            .iter()
            .filter(|range| range.start <= offset && offset < range.end)
            .min_by_key(|range| range.end - range.start)
            .map(|range| &range.origin)
    }
}

/// Translates `blueprint` to the StarScript code of its class.
///
/// # Errors
///
/// Returns the rules it breaks ([`validate`]), or what cannot be written as
/// StarScript: a value used where the node that computes it has not run
/// ([`BlueprintErrorKind::ValueNotReady`]) or a number that is not finite.
pub fn translate(blueprint: &Blueprint) -> Result<Translation, Vec<BlueprintError>> {
    validate(blueprint)?;
    let mut writer = SourceWriter::new();
    let mut errors = Vec::new();
    writer.line(HEADER);
    let class_start = writer.position();
    writer.indent();
    writer.write(&format!("class {}", blueprint.class));
    if let Some(base) = &blueprint.extends {
        writer.write(&format!(" extends {base}"));
    }
    writer.write(" {");
    writer.newline();
    writer.open();
    for variable in &blueprint.variables {
        write_variable(&mut writer, variable, &mut errors);
    }
    if !blueprint.base_arguments.is_empty() {
        write_constructor(&mut writer, &blueprint.base_arguments, &mut errors);
    }
    for graph in &blueprint.graphs {
        writer.newline();
        GraphTranslator::new(graph, &mut writer, &mut errors).translate();
    }
    writer.close();
    writer.line("}");
    writer.record(class_start, Origin::Class);
    if errors.is_empty() {
        let (source, origins) = writer.finish();
        Ok(Translation { source, origins })
    } else {
        Err(errors)
    }
}

/// `init() { super.init(arguments) }`
fn write_constructor(
    writer: &mut SourceWriter,
    arguments: &[Literal],
    errors: &mut Vec<BlueprintError>,
) {
    let arguments = arguments
        .iter()
        .map(|argument| {
            literal_code(argument).unwrap_or_else(|| {
                errors.push(class_error(BlueprintErrorKind::UnwritableNumber));
                String::new()
            })
        })
        .collect::<Vec<_>>()
        .join(", ");
    writer.newline();
    writer.line("init() {");
    writer.open();
    writer.line(&format!("super.init({arguments})"));
    writer.close();
    writer.line("}");
}

fn class_error(kind: BlueprintErrorKind) -> BlueprintError {
    BlueprintError {
        graph: None,
        node: None,
        kind,
    }
}

/// `[private ][export ]var name[: type][ = value]`
fn write_variable(
    writer: &mut SourceWriter,
    variable: &Variable,
    errors: &mut Vec<BlueprintError>,
) {
    let start = writer.position();
    writer.indent();
    if variable.private {
        writer.write("private ");
    }
    if variable.export {
        writer.write("export ");
    }
    writer.write(&format!("var {}", variable.name));
    if let Some(ty) = &variable.ty {
        writer.write(&format!(": {ty}"));
    }
    if let Some(value) = &variable.default {
        match literal_code(value) {
            Some(code) => writer.write(&format!(" = {code}")),
            None => errors.push(class_error(BlueprintErrorKind::UnwritableNumber)),
        }
    }
    writer.newline();
    writer.record(start, Origin::Variable(variable.name.clone()));
}

#[cfg(test)]
mod tests;
