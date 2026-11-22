//! Calls: methods (overridable or not), constructors, builtin methods and
//! the methods of engine classes.

use super::conversions::{RC, RUNTIME};
use super::expressions::self_reference;
use super::names::ident;
use super::statements::Body;
use super::{Result, internal};
use crate::ast::{Expr, ExprKind, Literal};
use crate::semantic::{
    BuiltinMethod, ClassId, ClassOrigin, INPUT_CLASS, ParameterInfo, Resolution, Type,
};

impl Body<'_> {
    /// `callee(arguments)`
    pub(super) fn call(&self, callee: &Expr, arguments: &[Expr]) -> Result<String> {
        match self.context.resolution(callee)? {
            Resolution::Constructor(class) => {
                let info = self.context.analysis.classes().class(class);
                let arguments = self.arguments(info.constructor_parameters(), arguments)?;
                Ok(format!(
                    "{}::new({arguments})",
                    self.context.class_path(class)
                ))
            }
            Resolution::Method { class, index } => {
                self.method_call(callee, class, index, arguments)
            }
            Resolution::Builtin(method) => self.builtin_call(callee, method, arguments),
            _ => Err(internal(
                "call of something that is not callable",
                callee.span,
            )),
        }
    }

    /// The arguments, each converted to its parameter's type.
    pub(super) fn arguments(
        &self,
        parameters: &[ParameterInfo],
        arguments: &[Expr],
    ) -> Result<String> {
        let codes = parameters
            .iter()
            .zip(arguments)
            .map(|(parameter, argument)| self.value_as(argument, &parameter.ty))
            .collect::<Result<Vec<_>>>()?;
        Ok(codes.join(", "))
    }

    fn method_call(
        &self,
        callee: &Expr,
        class: ClassId,
        index: usize,
        arguments: &[Expr],
    ) -> Result<String> {
        let info = self.context.analysis.classes().class(class);
        let method = info
            .methods
            .get(index)
            .ok_or_else(|| internal("unknown method", callee.span))?;
        if info.origin == ClassOrigin::Native && info.name == INPUT_CLASS {
            return self.input_call(callee, &method.name, arguments);
        }
        let arguments = self.arguments(&method.parameters, arguments)?;
        // Static methods of the engine are functions of the runtime; the
        // others belong to an engine class with a generated module.
        if info.origin == ClassOrigin::Native && method.is_static {
            return native_call(&info.name, &method.name, &arguments)
                .ok_or_else(|| internal("unknown engine method", callee.span));
        }
        let path = self.context.class_path(class);
        let name = ident(&method.name);
        if method.is_static {
            return Ok(format!("{path}::Methods::{name}({arguments})"));
        }
        let call = |receiver: &str, receiver_type: &Type| -> Result<String> {
            if method.is_private {
                let receiver = self.context.coerce(
                    receiver.to_owned(),
                    receiver_type,
                    &Type::Class(class),
                    callee.span,
                )?;
                let arguments = with_receiver(format!("&{receiver}"), &arguments);
                return Ok(format!("{path}::Methods::{name}({arguments})"));
            }
            // Through the trait of the class that introduced the method, so
            // that each object runs its own version.
            let owner = self
                .context
                .class_path(self.introducing_class(class, &method.name));
            let arguments = with_receiver(receiver.to_owned(), &arguments);
            Ok(format!("{owner}::Object::{name}({arguments})"))
        };
        match &callee.kind {
            ExprKind::Name(_) => call(&self_reference(), &Type::Class(self.class)),
            ExprKind::Member { object, .. } if matches!(object.kind, ExprKind::Super) => {
                // The base version, whatever the object overrides.
                let receiver = format!("&({RC}::clone(this_) as {path}::Ref)");
                Ok(format!(
                    "{path}::Methods::{name}({})",
                    with_receiver(receiver, &arguments)
                ))
            }
            ExprKind::Member {
                object, null_safe, ..
            } => {
                let receiver = self.expression(object)?;
                let receiver_type = self.context.type_of(object)?;
                if !null_safe {
                    return call(&receiver, receiver_type);
                }
                let inner = call("value_", &receiver_type.non_null())?;
                let adapter = if method.return_type.is_nullable() {
                    "and_then"
                } else {
                    "map"
                };
                Ok(format!("({receiver}).{adapter}(|value_| {inner})"))
            }
            _ => Err(internal("method called through an expression", callee.span)),
        }
    }

    /// The class that declared `method` first: `class` itself, or the base
    /// whose version it overrides.
    fn introducing_class(&self, class: ClassId, method: &str) -> ClassId {
        let classes = self.context.analysis.classes();
        let mut current = class;
        loop {
            let overrides = classes
                .class(current)
                .methods
                .iter()
                .any(|info| info.name == method && info.is_override);
            let base = classes.class(current).base;
            let found = base.and_then(|base| classes.find_member(base, method));
            match (overrides, found) {
                (true, Some(found)) => current = found.class,
                _ => return current,
            }
        }
    }

    fn builtin_call(
        &self,
        callee: &Expr,
        method: BuiltinMethod,
        arguments: &[Expr],
    ) -> Result<String> {
        let ExprKind::Member {
            object, null_safe, ..
        } = &callee.kind
        else {
            return Err(internal("builtin method without a receiver", callee.span));
        };
        let receiver = self.expression(object)?;
        let receiver_type = self.context.type_of(object)?.non_null();
        let argument = match (arguments.first(), builtin_parameter(method, &receiver_type)) {
            (Some(argument), Some(parameter)) => self.value_as(argument, &parameter)?,
            _ => String::new(),
        };
        let value = if *null_safe {
            "value_"
        } else {
            receiver.as_str()
        };
        let code = builtin_code(method, &receiver_type, value, &argument);
        if !null_safe {
            return Ok(code);
        }
        let adapter = if method == BuiltinMethod::Pop {
            "and_then"
        } else {
            "map"
        };
        Ok(format!("({receiver}).{adapter}(|value_| {code})"))
    }

    /// `Input.is_down(action)` reads the field `is_down` of the action's
    /// state; the same for `just_pressed` and `just_released`. Action names
    /// written as text are passed as they are: the game loop never allocates
    /// memory to ask for an action. `axis` is a function of the runtime,
    /// and the pointer's methods read a field of the pointer's state.
    fn input_call(&self, callee: &Expr, method: &str, arguments: &[Expr]) -> Result<String> {
        if let Some(field) = pointer_field(method) {
            return Ok(format!("{RUNTIME}::input::pointer().{field}"));
        }
        match arguments {
            [negative, positive] => Ok(format!(
                "{RUNTIME}::input::axis({}, {})",
                self.action_name(negative)?,
                self.action_name(positive)?
            )),
            [action] => Ok(format!(
                "{RUNTIME}::input::action_state({}).{method}",
                self.action_name(action)?
            )),
            _ => Err(internal("`Input` takes one or two actions", callee.span)),
        }
    }

    /// An action name for the runtime: text as it is, or a borrowed value.
    fn action_name(&self, argument: &Expr) -> Result<String> {
        Ok(match &argument.kind {
            ExprKind::Literal(Literal::String(text)) => format!("{text:?}"),
            _ => format!("&({})", self.value_as(argument, &Type::String)?),
        })
    }
}

/// `receiver, arguments…`
fn with_receiver(receiver: String, arguments: &str) -> String {
    if arguments.is_empty() {
        receiver
    } else {
        format!("{receiver}, {arguments}")
    }
}

/// Methods of the classes implemented by the engine.
fn native_call(class: &str, method: &str, arguments: &str) -> Option<String> {
    match (class, method) {
        ("Debug", "print") => Some(format!("{RUNTIME}::print(&({arguments}))")),
        ("Scene", "load") => Some(format!("crate::classes::Scene::load({arguments})")),
        ("Sound", "load") => Some(format!("crate::classes::Sound::load({arguments})")),
        ("Audio", "volume") => Some(format!("{RUNTIME}::audio::volume()")),
        ("Audio", "set_volume") => Some(format!("{RUNTIME}::audio::set_volume({arguments})")),
        _ => None,
    }
}

/// The field of the runtime's pointer state that the `Input` method
/// `method` reads, if it is one of the pointer's.
fn pointer_field(method: &str) -> Option<&'static str> {
    match method {
        "pointer_x" => Some("x"),
        "pointer_y" => Some("y"),
        "has_pointer" => Some("is_present"),
        _ => None,
    }
}

/// The type of the argument of a builtin method, if it takes one.
fn builtin_parameter(method: BuiltinMethod, receiver: &Type) -> Option<Type> {
    match (method, receiver) {
        (BuiltinMethod::Min | BuiltinMethod::Max, _) => Some(receiver.clone()),
        (BuiltinMethod::Contains | BuiltinMethod::Push, Type::List(element)) => {
            Some((**element).clone())
        }
        (BuiltinMethod::Contains, Type::String) => Some(Type::String),
        _ => None,
    }
}

/// A builtin method of `receiver` (a value of `receiver_type`).
fn builtin_code(
    method: BuiltinMethod,
    receiver_type: &Type,
    receiver: &str,
    argument: &str,
) -> String {
    use BuiltinMethod as Method;

    match (method, receiver_type) {
        (Method::Abs, Type::Int) => format!("({receiver}).wrapping_abs()"),
        (Method::Abs, _) => format!("({receiver}).abs()"),
        (Method::Min, _) => format!("({receiver}).min({argument})"),
        (Method::Max, _) => format!("({receiver}).max({argument})"),
        (Method::Floor, _) => format!("({receiver}).floor()"),
        (Method::Ceil, _) => format!("({receiver}).ceil()"),
        (Method::Round, _) => format!("({receiver}).round()"),
        (Method::Sqrt, _) => format!("({receiver}).sqrt()"),
        (Method::ToString, Type::Float) => format!("{RUNTIME}::float_to_string({receiver})"),
        (Method::ToString, Type::String) => receiver.to_owned(),
        (Method::ToString, _) => format!("({receiver}).to_string()"),
        (Method::Length, Type::String) => format!("(({receiver}).chars().count() as i64)"),
        (Method::Contains, Type::String) => format!("({receiver}).contains(({argument}).as_str())"),
        (Method::Contains, _) => format!("({receiver}).contains(&({argument}))"),
        (Method::ToUpper, _) => format!("({receiver}).to_uppercase()"),
        (Method::ToLower, _) => format!("({receiver}).to_lowercase()"),
        (Method::Length, _) => format!("({receiver}).length()"),
        (Method::IsEmpty, _) => format!("({receiver}).is_empty()"),
        (Method::Push, _) => format!("({receiver}).push({argument})"),
        (Method::Pop, _) => format!("({receiver}).pop()"),
        (Method::Clear, _) => format!("({receiver}).clear()"),
    }
}
