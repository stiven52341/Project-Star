//! The body of a function: its locals and statements.

use std::collections::HashMap;
use std::sync::Arc;

use star_script::ast::{
    AssignOperator, BinaryOperator, Block, ElseBranch, Expr, ExprKind, IfStatement, MethodDecl,
    Name, Parameter, Statement as SyntaxStatement, StatementKind, VariableDecl,
};
use star_script::semantic::{Analysis, ClassId, ClassOrigin, LocalId, Resolution, Type};

use super::Tables;
use crate::error::LowerError;
use crate::tree::{Arithmetic, EngineField, Expression, FieldRef, Function, Literal, Statement};

/// What `self` is in the code being lowered.
pub(super) enum Receiver {
    /// Nothing: a static function or a constant.
    None,
    /// The object, in this local.
    Object(usize),
    /// The start of `init`, before the object exists: assigning a field
    /// of the class puts its value in the local at the field's index.
    Setup(Vec<usize>),
}

/// Lowers the body of one function.
pub(super) struct Body<'a> {
    pub(super) analysis: &'a Analysis,
    pub(super) tables: &'a Tables,
    /// The class the code belongs to.
    pub(super) class: ClassId,
    /// What `self` is.
    pub(super) receiver: Receiver,
    /// The slot of each local declared so far.
    slots: HashMap<LocalId, usize>,
    /// How many slots are taken, the hidden ones included.
    slot_count: usize,
    /// What `return` gives back.
    return_type: Type,
    /// Whether the code is a `parallel` function's, where calls of other
    /// `parallel` functions run directly, on the same thread.
    pub(super) in_parallel: bool,
}

impl<'a> Body<'a> {
    pub(super) fn new(
        analysis: &'a Analysis,
        tables: &'a Tables,
        class: ClassId,
        return_type: Type,
    ) -> Self {
        Self {
            analysis,
            tables,
            class,
            receiver: Receiver::None,
            slots: HashMap::new(),
            slot_count: 0,
            return_type,
            in_parallel: false,
        }
    }

    /// The function with these parameters and body; a method of objects
    /// has `self` in its first local.
    pub(super) fn function(
        mut self,
        declaration: &MethodDecl,
        is_static: bool,
    ) -> Result<Function, LowerError> {
        if !is_static {
            self.receiver = Receiver::Object(self.hidden_slot());
        }
        for parameter in &declaration.parameters {
            self.parameter(parameter)?;
        }
        let body = self.block(&declaration.body)?;
        Ok(Function {
            slots: self.slot_count,
            body,
        })
    }

    pub(super) fn parameter(&mut self, parameter: &Parameter) -> Result<usize, LowerError> {
        self.declare(&parameter.name)
    }

    /// How many slots are taken.
    pub(super) const fn slot_count(&self) -> usize {
        self.slot_count
    }

    /// A slot no name of the program refers to.
    pub(super) const fn hidden_slot(&mut self) -> usize {
        self.slot_count += 1;
        self.slot_count - 1
    }

    /// A new slot for the local declared with `name`.
    fn declare(&mut self, name: &Name) -> Result<usize, LowerError> {
        let local = self
            .analysis
            .declared_local(name)
            .ok_or_else(|| LowerError::internal("local without analysis", name.span))?;
        let slot = self.hidden_slot();
        self.slots.insert(local, slot);
        Ok(slot)
    }

    /// `self`, where the object exists.
    pub(super) fn self_value(&self, expression: &Expr) -> Result<Expression, LowerError> {
        match self.receiver {
            Receiver::Object(slot) => Ok(Expression::Local(slot)),
            Receiver::None | Receiver::Setup(_) => Err(LowerError::internal(
                "`self` where there is no object",
                expression.span,
            )),
        }
    }

    /// Where the field (`class`, `index`) is in the objects of `class`
    /// and its subclasses: among the runtime's fields if `class` is an
    /// engine class, or else in the row of script fields, after those of
    /// its bases.
    pub(super) fn field_ref(
        &self,
        class: ClassId,
        index: usize,
        expression: &Expr,
    ) -> Result<FieldRef, LowerError> {
        let classes = self.analysis.classes();
        let info = classes.class(class);
        if info.origin == ClassOrigin::Native {
            let field = info
                .fields
                .get(index)
                .ok_or_else(|| LowerError::internal("unknown field", expression.span))?;
            return engine_field(&info.name, &field.name)
                .map(FieldRef::Engine)
                .ok_or_else(|| {
                    LowerError::unsupported("this field of the engine", expression.span)
                });
        }
        Ok(FieldRef::Script(script_field(self.analysis, class, index)))
    }

    /// The type of the field (`class`, `index`).
    pub(super) fn field_type(
        &self,
        class: ClassId,
        index: usize,
        expression: &Expr,
    ) -> Result<&'a Type, LowerError> {
        self.analysis
            .classes()
            .class(class)
            .fields
            .get(index)
            .map(|field| &field.ty)
            .ok_or_else(|| LowerError::internal("unknown field", expression.span))
    }

    /// The slot of a local declared before.
    pub(super) fn slot(&self, local: LocalId, expression: &Expr) -> Result<usize, LowerError> {
        self.slots.get(&local).copied().ok_or_else(|| {
            LowerError::internal("local used before its declaration", expression.span)
        })
    }

    /// The declared type of a local.
    pub(super) fn local_type(
        &self,
        local: LocalId,
        expression: &Expr,
    ) -> Result<&Type, LowerError> {
        self.analysis
            .local(local)
            .map(|info| &info.ty)
            .ok_or_else(|| LowerError::internal("unknown local", expression.span))
    }

    pub(super) fn block(&mut self, block: &Block) -> Result<Vec<Statement>, LowerError> {
        block
            .statements
            .iter()
            .map(|statement| self.statement(statement))
            .collect()
    }

    pub(super) fn statement(
        &mut self,
        statement: &SyntaxStatement,
    ) -> Result<Statement, LowerError> {
        Ok(match &statement.kind {
            StatementKind::Variable(variable) => self.variable(variable)?,
            StatementKind::Assignment {
                target,
                operator,
                value,
            } => self.assignment(target, *operator, value)?,
            StatementKind::Expression(expression) => {
                Statement::Evaluate(self.expression(expression)?)
            }
            StatementKind::If(if_statement) => self.if_statement(if_statement)?,
            StatementKind::While { condition, body } => Statement::While {
                condition: self.expression(condition)?,
                body: self.block(body)?,
            },
            StatementKind::For {
                variable,
                iterable,
                body,
            } => self.for_statement(variable, iterable, body)?,
            StatementKind::Return(value) => Statement::Return(
                value
                    .as_ref()
                    .map(|value| self.value_as(value, &self.return_type))
                    .transpose()?,
            ),
            StatementKind::Break => Statement::Break,
            StatementKind::Continue => Statement::Continue,
        })
    }

    fn variable(&mut self, variable: &VariableDecl) -> Result<Statement, LowerError> {
        let span = variable.name.span;
        let local = self
            .analysis
            .declared_local(&variable.name)
            .and_then(|id| self.analysis.local(id))
            .ok_or_else(|| LowerError::internal("variable without analysis", span))?;
        // Its value is lowered before the local exists: it cannot read it.
        let value = match &variable.initializer {
            Some(value) => self.value_as(value, &local.ty)?,
            None => default_value(&local.ty)
                .ok_or_else(|| LowerError::internal("type without a default value", span))?,
        };
        let slot = self.declare(&variable.name)?;
        Ok(Statement::Store { slot, value })
    }

    fn assignment(
        &self,
        target: &Expr,
        operator: AssignOperator,
        value: &Expr,
    ) -> Result<Statement, LowerError> {
        if let ExprKind::Index { object, index } = &target.kind {
            return Ok(Statement::SetElement {
                list: self.expression(object)?,
                index: self.expression(index)?,
                operator: compound_operator(operator),
                value: self.value_as(value, self.type_of(target)?)?,
            });
        }
        let local = match self.resolution(target)? {
            Some(Resolution::Local(local)) => local,
            Some(Resolution::Field { class, index }) => {
                return self.field_assignment(target, class, index, operator, value);
            }
            _ => {
                return Err(LowerError::internal(
                    "assignment to something that is not a variable",
                    target.span,
                ));
            }
        };
        let slot = self.slot(local, target)?;
        let ty = self.local_type(local, target)?.clone();
        let value = self.value_as(value, &ty)?;
        let value = match compound_operator(operator) {
            None => value,
            Some(operator) => Expression::Arithmetic {
                operator,
                left: Box::new(Expression::Local(slot)),
                right: Box::new(value),
            },
        };
        Ok(Statement::Store { slot, value })
    }

    /// `field = value` or `object.field = value`, also with `op=`.
    fn field_assignment(
        &self,
        target: &Expr,
        class: ClassId,
        index: usize,
        operator: AssignOperator,
        value: &Expr,
    ) -> Result<Statement, LowerError> {
        let value = self.value_as(value, self.field_type(class, index, target)?)?;
        let object = match &target.kind {
            ExprKind::Member { object, .. } if !matches!(object.kind, ExprKind::SelfValue) => {
                Some(&**object)
            }
            _ => None,
        };
        if let (Receiver::Setup(fields), None) = (&self.receiver, object) {
            // The object does not exist yet: keep the value to build it.
            let slot = fields
                .get(index)
                .copied()
                .filter(|_| class == self.class && operator == AssignOperator::Assign)
                .ok_or_else(|| {
                    LowerError::internal("field changed before the object exists", target.span)
                })?;
            return Ok(Statement::Store { slot, value });
        }
        let object = match object {
            Some(object) => self.expression(object)?,
            None => self.self_value(target)?,
        };
        Ok(Statement::SetField {
            object,
            field: self.field_ref(class, index, target)?,
            operator: compound_operator(operator),
            value,
        })
    }

    fn if_statement(&mut self, if_statement: &IfStatement) -> Result<Statement, LowerError> {
        let else_branch = match &if_statement.else_branch {
            None => Vec::new(),
            Some(ElseBranch::Block(block)) => self.block(block)?,
            Some(ElseBranch::If(nested)) => vec![self.if_statement(nested)?],
        };
        Ok(Statement::If {
            condition: self.expression(&if_statement.condition)?,
            then_branch: self.block(&if_statement.then_block)?,
            else_branch,
        })
    }

    fn for_statement(
        &mut self,
        variable: &Name,
        iterable: &Expr,
        body: &Block,
    ) -> Result<Statement, LowerError> {
        let ExprKind::Binary {
            operator: BinaryOperator::Range,
            left,
            right,
        } = &iterable.kind
        else {
            let list = self.expression(iterable)?;
            let slot = self.declare(variable)?;
            return Ok(Statement::ForEach {
                slot,
                list,
                body: self.block(body)?,
            });
        };
        let (start, end) = self.common_operands(left, right)?;
        let slot = self.declare(variable)?;
        Ok(Statement::ForRange {
            slot,
            start,
            end,
            body: self.block(body)?,
        })
    }
}

/// The place in the row of script fields of the field (`class`, `index`)
/// of a class of the program: after those of its bases.
pub(super) fn script_field(analysis: &Analysis, class: ClassId, index: usize) -> usize {
    let classes = analysis.classes();
    let before: usize = classes
        .ancestors(class)
        .skip(1)
        .filter(|base| classes.class(*base).origin != ClassOrigin::Native)
        .map(|base| classes.class(base).fields.len())
        .sum();
    before + index
}

/// The value a variable or field of `ty` starts with when none is given: an
/// expression, so each run of the declaration gets a list of its own.
pub(super) fn default_value(ty: &Type) -> Option<Expression> {
    let value = match ty {
        Type::Short => Literal::Short(0),
        Type::Int => Literal::Int(0),
        Type::BigInt => Literal::BigInt(0),
        Type::Float => Literal::Float(0.0),
        Type::Double => Literal::Double(0.0),
        Type::Bool => Literal::Bool(false),
        Type::String => Literal::String(Arc::from("")),
        Type::Nullable(_) => Literal::Null,
        Type::List(_) => return Some(Expression::List(Vec::new())),
        _ => return None,
    };
    Some(Expression::Constant(value))
}

/// The field `field` of the engine class `class`, if the interpreter
/// knows it.
pub(super) fn engine_field(class: &str, field: &str) -> Option<EngineField> {
    Some(match (class, field) {
        ("Color", "r") => EngineField::Red,
        ("Color", "g") => EngineField::Green,
        ("Color", "b") => EngineField::Blue,
        ("Color", "a") => EngineField::Alpha,
        ("Node", "name") => EngineField::Name,
        ("Node", "x") => EngineField::X,
        ("Node", "y") => EngineField::Y,
        ("Node", "rotation") => EngineField::Rotation,
        ("Node", "scale_x") => EngineField::ScaleX,
        ("Node", "scale_y") => EngineField::ScaleY,
        ("Node", "tags") => EngineField::Tags,
        ("Node", "visible") => EngineField::Visible,
        ("Animator", "animations") => EngineField::Animations,
        ("Animator", "states") => EngineField::States,
        ("Bone2D", "length") => EngineField::Length,
        ("Model", "file") => EngineField::ModelFile,
        ("Node", "body") => EngineField::Body,
        ("Node3D", "z") => EngineField::Z,
        ("Node3D", "pitch") => EngineField::Pitch,
        ("Node3D", "yaw") => EngineField::Yaw,
        ("Node3D", "scale_z") => EngineField::ScaleZ,
        ("Sprite" | "Mesh", "texture") => EngineField::Texture,
        ("Sprite" | "Body" | "Mesh", "width") => EngineField::Width,
        ("Sprite" | "Body" | "Mesh", "height") => EngineField::Height,
        ("Sprite", "layer") => EngineField::Layer,
        ("Sprite", "columns") => EngineField::Columns,
        ("Sprite", "rows") => EngineField::Rows,
        ("Sprite", "frame") => EngineField::Frame,
        ("Sprite", "frames") => EngineField::Frames,
        ("Sprite" | "Mesh", "pixelated") => EngineField::Pixelated,
        ("Sprite" | "Light2D" | "Light" | "Mesh", "color") => EngineField::Tint,
        ("Light2D" | "Light" | "Body", "kind") => EngineField::Kind,
        ("Body" | "Mesh", "shape") => EngineField::Shape,
        ("Body" | "Mesh", "radius") => EngineField::Radius,
        ("Body" | "Mesh", "depth") => EngineField::Depth,
        ("Mesh", "metallic") => EngineField::Metallic,
        ("Mesh", "roughness") => EngineField::Roughness,
        ("Body", "gravity_scale") => EngineField::GravityScale,
        ("Body", "friction") => EngineField::Friction,
        ("Body", "bounciness") => EngineField::Bounciness,
        ("Body", "density") => EngineField::Density,
        ("Body", "velocity_x") => EngineField::VelocityX,
        ("Body", "velocity_y") => EngineField::VelocityY,
        ("Body", "velocity_z") => EngineField::VelocityZ,
        ("Body", "fixed_rotation") => EngineField::FixedRotation,
        ("Body", "sensor") => EngineField::Sensor,
        ("Sound", "volume") => EngineField::Volume,
        ("Sound", "looping") => EngineField::Looping,
        ("TweenOptions", "delay") => EngineField::Delay,
        ("TweenOptions", "back") => EngineField::Back,
        ("TweenOptions", "repeats") => EngineField::Repeats,
        ("Light2D" | "Light", "intensity") => EngineField::Intensity,
        ("Light2D" | "Light", "range") => EngineField::Range,
        ("Light2D" | "Light", "angle") => EngineField::Angle,
        ("Light2D" | "Light", "blend") => EngineField::Blend,
        ("Camera2D", "view_height") => EngineField::ViewHeight,
        ("Camera2D" | "Camera3D", "main") => EngineField::Main,
        ("Camera3D", "fov") => EngineField::Fov,
        ("Game", "background") => EngineField::Background,
        ("Game", "camera") => EngineField::Camera,
        ("Game", "camera_3d") => EngineField::Camera3D,
        ("Game", "ambient") => EngineField::Ambient,
        ("Game", "ambient_2d") => EngineField::Ambient2D,
        _ => return None,
    })
}

/// The operator of `a op= b`, or `None` for plain `=`.
const fn compound_operator(operator: AssignOperator) -> Option<Arithmetic> {
    match operator {
        AssignOperator::Assign => None,
        AssignOperator::Add => Some(Arithmetic::Add),
        AssignOperator::Subtract => Some(Arithmetic::Subtract),
        AssignOperator::Multiply => Some(Arithmetic::Multiply),
        AssignOperator::Divide => Some(Arithmetic::Divide),
        AssignOperator::Remainder => Some(Arithmetic::Remainder),
    }
}
