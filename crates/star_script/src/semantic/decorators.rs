//! Decorators: `@name(arguments)` before a member.
//!
//! The engine's own tell the editor's inspector how to show an `export`
//! field: `@range(min, max)` or `@range(min, max, step)` for numbers,
//! `@tooltip("…")`, `@header("…")` and `@multiline` for texts. Their
//! arguments are plain values, read while checking.

use crate::ast::{ClassDecl, Decorator, Member, MemberKind, ModifierKind};
use crate::decorated::DECORATOR_CLASS;

use super::checker::Checker;
use super::field_defaults::{FieldDefault, plain_value};
use super::{ClassId, MemberRef, SemanticErrorKind, Type};

/// `@range`: numbers between two values, in steps or not.
pub const RANGE_DECORATOR: &str = "range";
/// `@tooltip`: the help the inspector shows over a field.
pub const TOOLTIP_DECORATOR: &str = "tooltip";
/// `@header`: a title over a field and those after it.
pub const HEADER_DECORATOR: &str = "header";
/// `@multiline`: a text edited in a box of several lines.
pub const MULTILINE_DECORATOR: &str = "multiline";
/// Every decorator of the engine, in the order the specification lists
/// them.
pub const ENGINE_DECORATORS: [&str; 4] = [
    RANGE_DECORATOR,
    TOOLTIP_DECORATOR,
    HEADER_DECORATOR,
    MULTILINE_DECORATOR,
];

/// How the inspector shows an `export` field, as its decorators say.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InspectorHints {
    /// `@range`: the values it can take.
    pub range: Option<NumberRange>,
    /// `@tooltip`: help shown over it.
    pub tooltip: Option<String>,
    /// `@header`: a title shown before it.
    pub header: Option<String>,
    /// `@multiline`: a box of several lines for its text.
    pub multiline: bool,
}

/// The numbers `@range(min, max, step)` allows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NumberRange {
    /// The smallest.
    pub min: f64,
    /// The largest.
    pub max: f64,
    /// How far apart the values are, if they are not continuous.
    pub step: Option<f64>,
}

impl Checker<'_> {
    /// Checks the decorators of the members of `declaration`, and keeps
    /// what those of its `export` fields say for the inspector.
    pub(super) fn check_decorators(&mut self, declaration: &ClassDecl, id: ClassId) {
        for member in &declaration.members {
            if member.decorators.is_empty() {
                continue;
            }
            let field = match &member.kind {
                MemberKind::Variable(variable) => {
                    match self
                        .analysis
                        .classes
                        .class(id)
                        .own_member(&variable.name.text)
                    {
                        Some(MemberRef::Field(index)) => Some(index),
                        _ => None,
                    }
                }
                MemberKind::Method(_) | MemberKind::Constructor(_) => None,
            };
            let exported = field.filter(|_| member.has_modifier(ModifierKind::Export));
            let hints = self.inspector_hints(id, member, exported);
            if let Some(index) = exported {
                self.analysis.classes.class_mut(id).fields[index].hints = hints;
            }
        }
    }

    /// What the decorators of `member` say for the inspector; `field` is
    /// the position of the `export` field it declares, if it is one.
    fn inspector_hints(
        &mut self,
        class: ClassId,
        member: &Member,
        field: Option<usize>,
    ) -> InspectorHints {
        let mut hints = InspectorHints::default();
        let mut seen: Vec<&str> = Vec::new();
        for decorator in &member.decorators {
            let name = decorator.name.text.as_str();
            if seen.contains(&name) {
                let kind = SemanticErrorKind::DuplicateDecorator(name.to_owned());
                self.error(kind, decorator.span);
                continue;
            }
            seen.push(name);
            if !ENGINE_DECORATORS.contains(&name) {
                self.check_program_decorator(member, decorator);
                continue;
            }
            let Some(index) = field else {
                let kind = SemanticErrorKind::DecoratorNeedsExport(name.to_owned());
                self.error(kind, decorator.span);
                continue;
            };
            let ty = self.analysis.classes.class(class).fields[index].ty.clone();
            self.read_engine_decorator(decorator, &ty, &mut hints);
        }
        hints
    }

    /// A decorator of the program: a class that extends `Decorator`, on a
    /// method that returns nothing and runs at once on an object.
    fn check_program_decorator(&mut self, member: &Member, decorator: &Decorator) {
        let name = &decorator.name.text;
        if self.decorator_class(name).is_none() {
            let kind = if self.analysis.classes.find(name).is_some() {
                SemanticErrorKind::NotADecorator(name.clone())
            } else {
                SemanticErrorKind::UnknownDecorator(name.clone())
            };
            self.error(kind, decorator.name.span);
            return;
        }
        let fits = match &member.kind {
            MemberKind::Method(method) => {
                method.return_type.is_none()
                    && ![
                        ModifierKind::Static,
                        ModifierKind::Async,
                        ModifierKind::Parallel,
                    ]
                    .iter()
                    .any(|modifier| member.has_modifier(*modifier))
            }
            MemberKind::Variable(_) | MemberKind::Constructor(_) => false,
        };
        if !fits {
            let kind = SemanticErrorKind::DecoratorNeedsMethod(name.clone());
            self.error(kind, decorator.span);
        }
    }

    /// The class called `name`, if it extends `Decorator`.
    pub(super) fn decorator_class(&self, name: &str) -> Option<ClassId> {
        let classes = &self.analysis.classes;
        let decorator = classes.find(DECORATOR_CLASS)?;
        let class = classes.find(name)?;
        (class != decorator && classes.is_subclass(class, decorator)).then_some(class)
    }

    /// Reads one of the engine's decorators on a field of type `ty` into
    /// `hints`, or says what is wrong with it.
    fn read_engine_decorator(
        &mut self,
        decorator: &Decorator,
        ty: &Type,
        hints: &mut InspectorHints,
    ) {
        let name = decorator.name.text.as_str();
        let values: Option<Vec<FieldDefault>> =
            decorator.arguments.iter().map(plain_value).collect();
        let expected = match name {
            RANGE_DECORATOR => {
                if !ty.is_numeric() && *ty != Type::Error {
                    return self.wrong_field(decorator, "a number");
                }
                hints.range = values.as_deref().and_then(number_range);
                (hints.range.is_none()).then_some(
                    "two or three numbers: the smallest value, the largest and, if wanted, a positive step",
                )
            }
            TOOLTIP_DECORATOR | HEADER_DECORATOR => {
                let text = match values.as_deref() {
                    Some([FieldDefault::Text(text)]) => Some(text.clone()),
                    _ => None,
                };
                let missing = text.is_none();
                if name == TOOLTIP_DECORATOR {
                    hints.tooltip = text;
                } else {
                    hints.header = text;
                }
                missing.then_some("one text")
            }
            _ => {
                if *ty != Type::String && *ty != Type::Error {
                    return self.wrong_field(decorator, "a `string`");
                }
                hints.multiline = true;
                (!decorator.arguments.is_empty()).then_some("no arguments")
            }
        };
        if let Some(expected) = expected {
            let kind = SemanticErrorKind::DecoratorArguments {
                decorator: name.to_owned(),
                expected,
            };
            self.error(kind, decorator.span);
        }
    }

    fn wrong_field(&mut self, decorator: &Decorator, needs: &'static str) {
        let kind = SemanticErrorKind::DecoratorField {
            decorator: decorator.name.text.clone(),
            needs,
        };
        self.error(kind, decorator.span);
    }
}

/// `min, max` or `min, max, step`, numbers with `min` below `max` and a
/// positive `step`.
fn number_range(values: &[FieldDefault]) -> Option<NumberRange> {
    let numbers: Vec<f64> = values.iter().map(number).collect::<Option<_>>()?;
    let (min, max, step) = match numbers.as_slice() {
        [min, max] => (*min, *max, None),
        [min, max, step] => (*min, *max, Some(*step)),
        _ => return None,
    };
    let valid = min < max && step.is_none_or(|step| step > 0.0);
    valid.then_some(NumberRange { min, max, step })
}

#[allow(clippy::cast_precision_loss)] // Limits written in code are small.
fn number(value: &FieldDefault) -> Option<f64> {
    match value {
        FieldDefault::Int(number) => Some(*number as f64),
        FieldDefault::Float(number) => Some(*number),
        _ => None,
    }
}
