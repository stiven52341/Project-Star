//! Constructors, in the two phases of the language: the fields' values
//! and the start of `init` before the object exists, without `self`; the
//! rest of `init` once it exists.

use star_script::ast::{ClassDecl, ConstructorDecl, Expr, MemberKind, Statement, StatementKind};
use star_script::semantic::{ClassInfo, FieldInfo, Resolution};

use super::body::{Body, Receiver, default_value};
use crate::error::LowerError;
use crate::tree::{ClassRef, Constructor, Statement as Step};

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
        let split = self.object_complete(statements)?;
        let (before, after) = statements.split_at(split);
        self.receiver = Receiver::Setup(fields.clone());
        for statement in before {
            setup.push(self.statement(statement)?);
        }
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
            self_slot,
            rest,
        })
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

    /// Where the object becomes complete: after the last statement that
    /// gives a pending field of the class its value (decision of the
    /// language: those assignments are written directly in `init`).
    fn object_complete(&self, statements: &[Statement]) -> Result<usize, LowerError> {
        let mut complete = 0;
        for (position, statement) in statements.iter().enumerate() {
            if self.assigns_pending_field(statement)? {
                complete = position + 1;
            }
        }
        Ok(complete)
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
