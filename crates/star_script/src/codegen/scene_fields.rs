//! `set_scene_fields_`, which `Scene.load` calls to give the `export`
//! fields of a node's object the values its scene writes.

use super::Context;
use super::conversions::{RC, RUNTIME};
use super::names::ident;
use super::writer::Writer;
use crate::semantic::{COLOR_CLASS, ClassId, ClassOrigin, FieldInfo, Type};

/// Parameter of `set_scene_fields_`: the class the scene names.
const CLASS_ARGUMENT: &str = "class_";
/// Parameter of `set_scene_fields_`: the name of the field.
const FIELD_ARGUMENT: &str = "field_";
/// Parameter of `set_scene_fields_`: the value the scene gives it.
const VALUE_ARGUMENT: &str = "value_";
/// The object, as an object of the class the scene names.
const OBJECT: &str = "this_";

/// Writes `set_scene_fields_`: for each class of the program with `export`
/// fields (its own or of its bases), a branch that checks each value and
/// sets it. Any other field is an error that stops the scene.
pub(super) fn scene_fields(context: Context<'_>, out: &mut Writer) {
    let classes = context.analysis.classes();
    out.line("");
    out.line("/// Gives the `export` fields of the object of a scene node the values");
    out.line("/// its scene writes; fails if one is not such a field, or does not fit it.");
    out.open(&format!(
        "pub fn set_scene_fields_({CLASS_ARGUMENT}: &str, object_: &crate::classes::Node::Ref, values_: &[(::std::string::String, {RUNTIME}::scene_fields::SceneValue)]) -> ::std::result::Result<(), ::std::string::String> {{"
    ));
    out.open(&format!(
        "for ({FIELD_ARGUMENT}, {VALUE_ARGUMENT}) in values_ {{"
    ));
    out.open(&format!("match {CLASS_ARGUMENT} {{"));
    for (id, class) in classes.iter() {
        if class.origin == ClassOrigin::Native {
            continue;
        }
        let fields = exported_fields(context, id);
        if fields.is_empty() {
            continue;
        }
        out.open(&format!("{:?} => {{", class.name));
        out.line(&format!(
            "let {OBJECT} = {}({RC}::clone(object_)).ok_or_else(|| {RUNTIME}::scene_fields::unknown_field({CLASS_ARGUMENT}, {FIELD_ARGUMENT}))?;",
            context.cast_function(id)
        ));
        out.open(&format!("match {FIELD_ARGUMENT}.as_str() {{"));
        for (owner, field) in fields {
            out.line(&format!(
                "{:?} => {},",
                field.name,
                set_field(context, owner, field)
            ));
        }
        out.line(&format!(
            "_ => return ::std::result::Result::Err({RUNTIME}::scene_fields::unknown_field({CLASS_ARGUMENT}, {FIELD_ARGUMENT})),"
        ));
        out.close("}");
        out.close("}");
    }
    out.line(&format!(
        "_ => return ::std::result::Result::Err({RUNTIME}::scene_fields::unknown_field({CLASS_ARGUMENT}, {FIELD_ARGUMENT})),"
    ));
    out.close("}");
    out.close("}");
    out.line("::std::result::Result::Ok(())");
    out.close("}");
}

/// The `export` fields of `class` and its bases, each with the class that
/// declares it.
fn exported_fields(context: Context<'_>, class: ClassId) -> Vec<(ClassId, &FieldInfo)> {
    let classes = context.analysis.classes();
    classes
        .ancestors(class)
        .flat_map(|owner| {
            classes
                .class(owner)
                .fields
                .iter()
                .filter(|field| field.is_exported)
                .map(move |field| (owner, field))
        })
        .collect()
}

/// The code that sets `field`, declared by `owner`, to the value the
/// scene gives, converted to its type.
fn set_field(context: Context<'_>, owner: ClassId, field: &FieldInfo) -> String {
    let classes = context.analysis.classes();
    let target = format!(
        "{}::HasFields::fields(&*{OBJECT}).{}",
        context.class_path(owner),
        ident(&field.name)
    );
    let read = |function: &str| {
        format!("{RUNTIME}::scene_fields::{function}({FIELD_ARGUMENT}, {VALUE_ARGUMENT})?")
    };
    let function = match &field.ty {
        Type::Short => "short",
        Type::Int => "int",
        Type::BigInt => "bigint",
        Type::Float => "float",
        Type::Double => "double",
        Type::Bool => "boolean",
        Type::String => "text",
        Type::Class(id) if classes.class(*id).name == COLOR_CLASS => {
            let color = context.class_path(*id);
            return format!(
                "{{ let [r_, g_, b_, a_] = {}; {target}.set({color}::new(r_, g_, b_, a_)) }}",
                read("color")
            );
        }
        // Analysis lets only the types above be exported.
        _ => {
            return format!(
                "return ::std::result::Result::Err({RUNTIME}::scene_fields::unknown_field({CLASS_ARGUMENT}, {FIELD_ARGUMENT}))"
            );
        }
    };
    format!("{target}.set({})", read(function))
}
