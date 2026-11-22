//! Construction of objects, in the two phases the language requires.
//!
//! `init_` of a class receives `make_`, which builds the final object from
//! the fields of every class in its hierarchy. A subclass first prepares its
//! own fields (phase one, without `self`), then calls the `init_` of its base
//! with a `make_` that adds them. When the root class has its fields too, the
//! object is built, and each `init_` runs the rest of its body (phase two,
//! with `self`), from the root down. So the base `init` can already call
//! methods that a subclass overrides: the whole object exists.

use super::conversions::{RC, RUNTIME, default_value};
use super::names::ident;
use super::program::{ClassItem, parameter_list};
use super::statements::{Body, BodyKind, setup_field};
use super::writer::Writer;
use super::{Context, Result, internal};
use crate::ast::{Expr, ExprKind, Statement, StatementKind};
use crate::semantic::{ClassId, FieldInfo, Resolution, Type};

impl Context<'_> {
    /// `init_` and `new` of a class.
    pub(super) fn constructor(self, out: &mut Writer, item: &ClassItem<'_>) -> Result<()> {
        let hierarchy = self.hierarchy(item.id);
        let fields_types: Vec<String> = hierarchy
            .iter()
            .map(|class| format!("{}::Fields", self.class_path(*class)))
            .collect();
        let class = self.analysis.classes().class(item.id);
        let parameters = parameter_list(self, class.constructor_parameters(), item.span())?;
        let separator = if parameters.is_empty() { "" } else { ", " };

        out.line("/// Prepares an object of this class: `make_` builds it from the fields of");
        out.line("/// each class of its hierarchy, from the root down.");
        out.open(&format!(
            "pub fn init_<O: Object + 'static>({parameters}{separator}make_: impl FnOnce({}) -> {RC}<O>) -> {RC}<O> {{",
            fields_types.join(", ")
        ));
        self.constructor_body(out, item, &hierarchy)?;
        out.close("}");
        out.line("");

        let names: Vec<String> = class
            .constructor_parameters()
            .iter()
            .map(|parameter| ident(&parameter.name))
            .collect();
        let pieces: Vec<String> = (0..hierarchy.len())
            .map(|i| format!("fields{i}_"))
            .collect();
        let instance_fields: Vec<String> = hierarchy
            .iter()
            .zip(&pieces)
            .map(|(class, piece)| format!("{}: {piece}", self.member_name(*class)))
            .collect();
        out.line("/// Creates an object of this class.");
        out.open(&format!("pub fn new({parameters}) -> Ref {{"));
        out.line(&format!(
            "init_({}{separator}|{}| {RC}::new(Instance {{ {} }}))",
            names.join(", "),
            pieces.join(", "),
            instance_fields.join(", ")
        ));
        out.close("}");
        Ok(())
    }

    /// The classes of the hierarchy of `class`, from the root down to it.
    pub(super) fn hierarchy(self, class: ClassId) -> Vec<ClassId> {
        let mut hierarchy: Vec<ClassId> = self.analysis.classes().ancestors(class).collect();
        hierarchy.reverse();
        hierarchy
    }

    /// The name of a class as a Rust field or method name.
    pub(super) fn member_name(self, class: ClassId) -> String {
        ident(&self.analysis.classes().class(class).name)
    }

    fn constructor_body(
        self,
        out: &mut Writer,
        item: &ClassItem<'_>,
        hierarchy: &[ClassId],
    ) -> Result<()> {
        let statements = item
            .constructor
            .map_or(&[][..], |constructor| &constructor.body.statements);
        let split = self.split(item.id, statements)?;
        let setup = Body {
            context: self,
            class: item.id,
            kind: BodyKind::ConstructorSetup,
            return_type: Type::Void,
        };
        self.initial_fields(out, item, &setup)?;
        setup.statements(out, &statements[..split.setup_end])?;
        self.build_fields(out, item);
        self.build_object(out, item.id, hierarchy, &setup, split.super_arguments)?;
        out.line(&format!(
            "let reference_: Ref = {RC}::clone(&object_) as Ref;"
        ));
        out.line("let this_ = &reference_;");
        let body = Body {
            kind: BodyKind::Constructor,
            ..setup
        };
        body.statements(out, &statements[split.rest_start..])?;
        out.line("object_");
        Ok(())
    }

    /// The fields that do not wait for `init`: their initial or default value.
    fn initial_fields(self, out: &mut Writer, item: &ClassItem<'_>, body: &Body<'_>) -> Result<()> {
        let class = self.analysis.classes().class(item.id);
        for (index, field) in class.fields.iter().enumerate() {
            if field.needs_constructor() {
                continue;
            }
            let ty = self.rust_type(&field.ty, field.span)?;
            let value = match item.field_initializer(field.span) {
                Some(initializer) => body.value_as(initializer, &field.ty)?,
                None => default_value(&field.ty, field.span)?,
            };
            out.line(&format!("let {}: {ty} = {value};", setup_field(index)));
        }
        Ok(())
    }

    fn build_fields(self, out: &mut Writer, item: &ClassItem<'_>) {
        let class = self.analysis.classes().class(item.id);
        let fields: Vec<String> = class
            .fields
            .iter()
            .enumerate()
            .map(|(index, field)| {
                format!(
                    "{}: {RUNTIME}::Field::new({})",
                    ident(&field.name),
                    setup_field(index)
                )
            })
            .collect();
        if fields.is_empty() {
            out.line("let fields_ = Fields {};");
        } else {
            out.line(&format!(
                "let fields_ = Fields {{ {} }};",
                fields.join(", ")
            ));
        }
    }

    /// Builds the object: directly in a root class, through the base's
    /// `init_` in a subclass.
    fn build_object(
        self,
        out: &mut Writer,
        class: ClassId,
        hierarchy: &[ClassId],
        setup: &Body<'_>,
        super_arguments: Option<&[Expr]>,
    ) -> Result<()> {
        let Some(base) = self.analysis.classes().class(class).base else {
            out.line("let object_ = make_(fields_);");
            return Ok(());
        };
        let parameters = self.analysis.classes().class(base).constructor_parameters();
        let arguments = setup.arguments(parameters, super_arguments.unwrap_or_default())?;
        let separator = if arguments.is_empty() { "" } else { ", " };
        let pieces: Vec<String> = (0..hierarchy.len() - 1)
            .map(|i| format!("base{i}_"))
            .collect();
        out.line(&format!(
            "let object_ = {}::init_({arguments}{separator}move |{}| make_({}, fields_));",
            self.class_path(base),
            pieces.join(", "),
            pieces.join(", ")
        ));
        Ok(())
    }

    /// Where the object becomes complete: at `super.init(…)`, or after the
    /// last statement that gives a pending field its value.
    fn split(self, class: ClassId, statements: &[Statement]) -> Result<Split<'_>> {
        for (position, statement) in statements.iter().enumerate() {
            if let Some(arguments) = self.super_init_arguments(statement)? {
                return Ok(Split {
                    setup_end: position,
                    super_arguments: Some(arguments),
                    rest_start: position + 1,
                });
            }
        }
        let mut setup_end = 0;
        for (position, statement) in statements.iter().enumerate() {
            if self.assigns_pending_field(class, statement)? {
                setup_end = position + 1;
            }
        }
        Ok(Split {
            setup_end,
            super_arguments: None,
            rest_start: setup_end,
        })
    }

    fn super_init_arguments(self, statement: &Statement) -> Result<Option<&[Expr]>> {
        let StatementKind::Expression(Expr {
            kind: ExprKind::Call { callee, arguments },
            ..
        }) = &statement.kind
        else {
            return Ok(None);
        };
        let is_super_init = matches!(
            self.info(callee)?.resolution,
            Some(Resolution::SuperConstructor(_))
        );
        Ok(is_super_init.then_some(arguments.as_slice()))
    }

    fn assigns_pending_field(self, class: ClassId, statement: &Statement) -> Result<bool> {
        let StatementKind::Assignment { target, .. } = &statement.kind else {
            return Ok(false);
        };
        let Some(Resolution::Field {
            class: owner,
            index,
        }) = self.info(target)?.resolution
        else {
            return Ok(false);
        };
        let fields = &self.analysis.classes().class(class).fields;
        let pending = fields.get(index).is_some_and(FieldInfo::needs_constructor);
        if owner == class && pending {
            Ok(true)
        } else if owner == class && fields.get(index).is_none() {
            Err(internal("unknown field", target.span))
        } else {
            Ok(false)
        }
    }
}

/// The parts of an `init` body.
struct Split<'s> {
    /// Statements before this position run before the object exists.
    setup_end: usize,
    /// Arguments of `super.init(…)`, if it is called explicitly.
    super_arguments: Option<&'s [Expr]>,
    /// Statements from this position run once the object exists.
    rest_start: usize,
}
