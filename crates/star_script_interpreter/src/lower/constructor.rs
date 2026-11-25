//! Constructors, in the two phases of the language: the fields' values
//! and the start of `init` before the object exists, without `self`; the
//! rest of `init` once it exists.

use star_script::ast::{
    ClassDecl, ConstructorDecl, Expr, ExprKind, MemberKind, Statement, StatementKind,
};
use star_script::semantic::{ClassInfo, FieldInfo, Resolution};

use super::body::{Body, Receiver, default_value};
use crate::error::LowerError;
use crate::tree::{BaseConstructor, ClassRef, Constructor, Statement as Step};

impl Body<'_> {
    /// The constructor of `info`, whose objects are of the class `class`.
    pub(super) fn constructor(
        mut self,
        class: ClassRef,
        info: &ClassInfo,
        declaration: &ClassDecl,
    ) -> Result<Constructor, LowerError> {
        let init = own_init(declaration);
        if let Some(init) = init {
            for parameter in &init.parameters {
                self.parameter(parameter)?;
            }
        }
        let mut setup = Vec::new();
        let mut fields = Vec::new();
        for field in &info.fields {
            let slot = self.hidden_slot();
            if let Some(value) = self.initial_value(declaration, field)? {
                setup.push(Step::Store { slot, value });
            }
            fields.push(slot);
        }
        let statements = init.map_or(&[][..], |init| &init.body.statements);
        let split = self.split(statements)?;
        self.receiver = Receiver::Setup(fields.clone());
        for statement in &statements[..split.setup_end] {
            setup.push(self.statement(statement)?);
        }
        let base = self.base_constructor(info, split.super_arguments, declaration)?;
        let after = &statements[split.rest_start..];
        let self_slot = self.hidden_slot();
        self.receiver = Receiver::Object(self_slot);
        let rest = after
            .iter()
            .map(|statement| self.statement(statement))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Constructor {
            class,
            slots: self.slot_count(),
            setup,
            fields,
            base,
            self_slot,
            rest,
        })
    }

    /// The call of the base's constructor, with the arguments of
    /// `super.init(…)` (none if it is not written).
    fn base_constructor(
        &self,
        info: &ClassInfo,
        arguments: Option<&[Expr]>,
        declaration: &ClassDecl,
    ) -> Result<Option<BaseConstructor>, LowerError> {
        let Some(base) = info.base else {
            return Ok(None);
        };
        let span = declaration.name.span;
        let constructor = self
            .tables
            .constructor(base)
            .ok_or_else(|| LowerError::internal("base without a constructor", span))?;
        let parameters = self.analysis.classes().class(base).constructor_parameters();
        let arguments = parameters
            .iter()
            .zip(arguments.unwrap_or_default())
            .map(|(parameter, argument)| self.value_as(argument, &parameter.ty))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Some(BaseConstructor {
            constructor,
            arguments,
        }))
    }

    /// The value a field starts with: the one written in the class, the
    /// type's default value, or none if `init` must give it one.
    fn initial_value(
        &self,
        declaration: &ClassDecl,
        field: &FieldInfo,
    ) -> Result<Option<crate::tree::Expression>, LowerError> {
        if let Some(value) = field_initializer(declaration, field) {
            return self.value_as(value, &field.ty).map(Some);
        }
        if field.needs_constructor() {
            return Ok(None);
        }
        default_value(&field.ty)
            .map(Some)
            .ok_or_else(|| LowerError::internal("field without a default value", field.span))
    }

    /// Where the object becomes complete, as compiled: at
    /// `super.init(…)`, or after the last statement that gives a pending
    /// field of the class its value (the language requires those
    /// assignments directly in `init`).
    fn split<'s>(&self, statements: &'s [Statement]) -> Result<Split<'s>, LowerError> {
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
            if self.assigns_pending_field(statement)? {
                setup_end = position + 1;
            }
        }
        Ok(Split {
            setup_end,
            super_arguments: None,
            rest_start: setup_end,
        })
    }

    /// The arguments of `statement` if it is `super.init(…)`.
    fn super_init_arguments<'s>(
        &self,
        statement: &'s Statement,
    ) -> Result<Option<&'s [Expr]>, LowerError> {
        let StatementKind::Expression(Expr {
            kind: ExprKind::Call { callee, arguments },
            ..
        }) = &statement.kind
        else {
            return Ok(None);
        };
        let is_super_init = matches!(
            self.resolution(callee)?,
            Some(Resolution::SuperConstructor(_))
        );
        Ok(is_super_init.then_some(arguments.as_slice()))
    }

    fn assigns_pending_field(&self, statement: &Statement) -> Result<bool, LowerError> {
        let StatementKind::Assignment { target, .. } = &statement.kind else {
            return Ok(false);
        };
        let Some(Resolution::Field { class, index }) = self.resolution(target)? else {
            return Ok(false);
        };
        let fields = &self.analysis.classes().class(self.class).fields;
        Ok(class == self.class && fields.get(index).is_some_and(FieldInfo::needs_constructor))
    }
}

/// The parts of an `init` body.
struct Split<'s> {
    /// Statements before this position run before the object exists.
    setup_end: usize,
    /// The arguments of `super.init(…)`, if it is written.
    super_arguments: Option<&'s [Expr]>,
    /// Statements from this position run once the object exists.
    rest_start: usize,
}

/// The `init` the class declares, if any.
fn own_init(declaration: &ClassDecl) -> Option<&ConstructorDecl> {
    declaration
        .members
        .iter()
        .find_map(|member| match &member.kind {
            MemberKind::Constructor(init) => Some(init),
            _ => None,
        })
}

/// The value written in the class for `field`, if any.
fn field_initializer<'a>(declaration: &'a ClassDecl, field: &FieldInfo) -> Option<&'a Expr> {
    declaration
        .members
        .iter()
        .find_map(|member| match &member.kind {
            MemberKind::Variable(variable) if variable.name.span == field.span => {
                variable.initializer.as_ref()
            }
            _ => None,
        })
}
