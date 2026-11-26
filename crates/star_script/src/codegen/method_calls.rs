//! `call_method_`, which animations call to run a method of a node by its
//! name (decision 79): the methods of the program's classes that take no
//! arguments, each object running its own version.

use super::Context;
use super::conversions::RC;
use super::names::ident;
use super::writer::Writer;
use crate::semantic::{ClassOrigin, Execution, MethodInfo};

/// Parameter of `call_method_`: the name of the method.
const METHOD_ARGUMENT: &str = "method_";
/// The object, as an object of the class that declares the method.
const OBJECT: &str = "this_";

/// Writes `call_method_`: for each class of the program, a branch that
/// runs the methods it introduces that an animation can call, if the
/// object is of that class. They run through the class's trait, so a
/// subclass's override runs for its objects. Any other method is an
/// error, which the animation warns about.
pub(super) fn method_caller(context: Context<'_>, out: &mut Writer) {
    let classes = context.analysis.classes();
    out.line("");
    out.line("/// Runs the method named of an object of any class of the program, if it");
    out.line("/// takes no arguments; fails if its class has no such method.");
    out.open(&format!(
        "pub fn call_method_(object_: &crate::classes::Node::Ref, {METHOD_ARGUMENT}: &str) -> ::std::result::Result<(), ::std::string::String> {{"
    ));
    for (id, class) in classes.iter() {
        if class.origin == ClassOrigin::Native {
            continue;
        }
        let callable: Vec<&MethodInfo> = class
            .methods
            .iter()
            .filter(|method| is_callable(method))
            .collect();
        if callable.is_empty() {
            continue;
        }
        let path = context.class_path(id);
        out.open(&format!(
            "if let ::std::option::Option::Some({OBJECT}) = {}({RC}::clone(object_)) {{",
            context.cast_function(id)
        ));
        out.open(&format!("match {METHOD_ARGUMENT} {{"));
        for method in callable {
            out.line(&format!(
                "{:?} => {{ let _ = {path}::Object::{}({RC}::clone(&{OBJECT})); return ::std::result::Result::Ok(()); }}",
                method.name,
                ident(&method.name)
            ));
        }
        out.line("_ => {}");
        out.close("}");
        out.close("}");
    }
    out.line(&format!(
        "::std::result::Result::Err(::std::format!(\"the node has no method `{{}}` that takes no arguments\", {METHOD_ARGUMENT}))"
    ));
    out.close("}");
}

/// Whether an animation can call `method` through the class that declares
/// it: one of its objects' own methods, public, that runs at once, takes no
/// arguments and is not an override (the class that introduced it calls
/// it).
fn is_callable(method: &MethodInfo) -> bool {
    !method.is_static
        && !method.is_private
        && !method.is_override
        && method.execution == Execution::Direct
        && method.parameters.is_empty()
}
