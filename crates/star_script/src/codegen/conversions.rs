//! Rust types, default values and the conversions between StarScript types
//! that Rust needs spelled out: `T` → `T?`, subclass → base, narrowing.

use super::names::ident;
use super::{Context, Result, internal};
use crate::Span;
use crate::semantic::{ClassId, Type};

/// Path of the runtime crate in generated code.
pub(super) const RUNTIME: &str = "::star_script_runtime";
/// Shared reference type of objects.
pub(super) const RC: &str = "::std::rc::Rc";
/// `Some` of `Option`, by a path no StarScript name can shadow.
pub(super) const SOME: &str = "::std::option::Option::Some";
/// `None` of `Option`.
pub(super) const NONE: &str = "::std::option::Option::None";
/// Trait implemented by every object, to ask for its class.
pub(super) const ANY_OBJECT: &str = "crate::AnyObject";

impl Context<'_> {
    /// Path of the module generated for a class: `crate::classes::Player`.
    pub(super) fn class_path(self, class: ClassId) -> String {
        let name = &self.analysis.classes().class(class).name;
        format!("crate::classes::{}", ident(name))
    }

    /// The method of [`ANY_OBJECT`] that converts an object to `class`.
    pub(super) fn cast_function(self, class: ClassId) -> String {
        let name = &self.analysis.classes().class(class).name;
        format!("{ANY_OBJECT}::{}", ident(name))
    }

    /// The Rust type of values of `ty`.
    pub(super) fn rust_type(self, ty: &Type, span: Span) -> Result<String> {
        Ok(match ty {
            Type::Short | Type::Int | Type::BigInt | Type::Float | Type::Double => {
                number_type(ty).unwrap_or_default().to_owned()
            }
            Type::Bool => "bool".to_owned(),
            Type::String => "::std::string::String".to_owned(),
            Type::Void => "()".to_owned(),
            Type::Class(class) => format!("{}::Ref", self.class_path(*class)),
            Type::List(element) => format!("{RUNTIME}::List<{}>", self.rust_type(element, span)?),
            Type::Nullable(inner) => {
                format!("::std::option::Option<{}>", self.rust_type(inner, span)?)
            }
            Type::Null | Type::Range | Type::Error => {
                return Err(internal("value without a concrete type", span));
            }
        })
    }

    /// `code`, a value of type `from`, as a value of type `to`.
    ///
    /// Semantic analysis already checked that `from` is assignable to `to`;
    /// this only adds what Rust needs to see the conversion.
    pub(super) fn coerce(self, code: String, from: &Type, to: &Type, span: Span) -> Result<String> {
        match (from, to) {
            _ if from == to => Ok(code),
            (Type::Null, Type::Nullable(_)) => Ok(code),
            (Type::Nullable(from), Type::Nullable(to)) => {
                let inner = self.coerce("value_".to_owned(), from, to, span)?;
                Ok(format!("({code}).map(|value_| {inner})"))
            }
            (from, Type::Nullable(to)) => {
                let inner = self.coerce(code, from, to, span)?;
                Ok(format!("{SOME}({inner})"))
            }
            // A wider number holds the value exactly: `as` only widens it.
            (from, to) if from.widens_to(to) => Ok(format!(
                "(({code}) as {})",
                number_type(to).unwrap_or_default()
            )),
            (Type::Class(_), Type::Class(to)) => {
                Ok(format!("(({code}) as {}::Ref)", self.class_path(*to)))
            }
            _ => Err(internal("conversion between unrelated types", span)),
        }
    }

    /// A local of type `declared` read where analysis proved it has the
    /// narrower type `known`: not `null` after `x != null`, or of a
    /// subclass after `x is Player`.
    pub(super) fn narrow(
        self,
        code: String,
        declared: &Type,
        known: &Type,
        span: Span,
    ) -> Result<String> {
        if declared == known {
            return Ok(code);
        }
        let mut code = code;
        let mut current = declared.clone();
        if current.is_nullable() && !known.is_nullable() {
            code = format!("{RUNTIME}::proven({code})");
            current = current.non_null();
        }
        match (current.class(), known.class()) {
            (Some(from), Some(to)) if from != to => {
                if self.analysis.classes().is_subclass(from, to) {
                    self.coerce(code, &current, known, span)
                } else {
                    let cast = self.downcast(&code, &current, to);
                    Ok(format!("{RUNTIME}::proven({cast})"))
                }
            }
            _ => Ok(code),
        }
    }

    /// `code` (an object of type `from`, maybe `null`) as an object of the
    /// subclass `to`, or `None` if it is not one.
    pub(super) fn downcast(self, code: &str, from: &Type, to: ClassId) -> String {
        let cast = self.cast_function(to);
        if from.is_nullable() {
            format!("({code}).and_then(|value_| {cast}(value_))")
        } else {
            format!("{cast}({code})")
        }
    }
}

/// The value a variable of `ty` starts with when none is given.
pub(super) fn default_value(ty: &Type, span: Span) -> Result<String> {
    Ok(match ty {
        Type::Float => "0.0_f32".to_owned(),
        Type::Double => "0.0_f64".to_owned(),
        integer if integer.is_integer() => {
            format!("0_{}", number_type(integer).unwrap_or_default())
        }
        Type::Bool => "false".to_owned(),
        Type::String => "::std::string::String::new()".to_owned(),
        Type::List(_) => format!("{RUNTIME}::List::new()"),
        Type::Nullable(_) => NONE.to_owned(),
        _ => return Err(internal("type without a default value", span)),
    })
}

/// Whether reading a value of `ty` must copy it with `.clone()`.
pub(super) const fn needs_clone(ty: &Type) -> bool {
    !(ty.is_numeric() || matches!(ty, Type::Bool))
}

/// The Rust type of a number type: `short` is `i16`, `float` is `f32`.
pub(super) const fn number_type(ty: &Type) -> Option<&'static str> {
    match ty {
        Type::Short => Some("i16"),
        Type::Int => Some("i32"),
        Type::BigInt => Some("i64"),
        Type::Float => Some("f32"),
        Type::Double => Some("f64"),
        _ => None,
    }
}

/// The code that reads `name` (a variable) as an owned value.
pub(super) fn read_variable(name: &str, ty: &Type) -> String {
    if needs_clone(ty) {
        format!("{name}.clone()")
    } else {
        name.to_owned()
    }
}
