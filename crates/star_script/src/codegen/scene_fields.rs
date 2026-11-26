//! `set_scene_fields_`, which `Scene.load` calls to give the `export`
//! fields of a node's object the values its scene writes, and
//! `set_exported_field_` and `exported_field_`, which animations and tweens
//! use to change and read one by name.

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

/// Writes `set_exported_field_`, which animations call: for each class of
/// the program that declares `export` fields, a branch that sets the one
/// named if the object is of that class. A field no class of the object
/// declares is an error, which the animation warns about.
pub(super) fn exported_field_setter(context: Context<'_>, out: &mut Writer) {
    let classes = context.analysis.classes();
    out.line("");
    out.line("/// Gives the `export` field named of an object of any class of the program");
    out.line("/// a value; fails if its class has no such field, or the value does not fit it.");
    out.open(&format!(
        "pub fn set_exported_field_(object_: &crate::classes::Node::Ref, {FIELD_ARGUMENT}: &str, {VALUE_ARGUMENT}: &{RUNTIME}::scene_fields::SceneValue) -> ::std::result::Result<(), ::std::string::String> {{"
    ));
    for (id, class) in classes.iter() {
        if class.origin == ClassOrigin::Native {
            continue;
        }
        let own: Vec<&FieldInfo> = class
            .fields
            .iter()
            .filter(|field| field.is_exported)
            .collect();
        if own.is_empty() {
            continue;
        }
        out.open(&format!(
            "if let ::std::option::Option::Some({OBJECT}) = {}({RC}::clone(object_)) {{",
            context.cast_function(id)
        ));
        out.open(&format!("match {FIELD_ARGUMENT} {{"));
        for field in own {
            out.line(&format!(
                "{:?} => {{ {}; return ::std::result::Result::Ok(()); }}",
                field.name,
                set_field(context, id, field)
            ));
        }
        out.line("_ => {}");
        out.close("}");
        out.close("}");
    }
    out.line(&format!(
        "::std::result::Result::Err({RUNTIME}::scene_fields::no_property({FIELD_ARGUMENT}))"
    ));
    out.close("}");
}

/// Writes `exported_field_`, which tweens call to read where a property
/// starts: for each class of the program that declares `export` fields, a
/// branch that reads the one named if the object is of that class.
pub(super) fn exported_field_getter(context: Context<'_>, out: &mut Writer) {
    let classes = context.analysis.classes();
    out.line("");
    out.line("/// The value of the `export` field named of an object of any class of the");
    out.line("/// program; fails if its class has no such field.");
    out.open(&format!(
        "pub fn exported_field_(object_: &crate::classes::Node::Ref, {FIELD_ARGUMENT}: &str) -> ::std::result::Result<{RUNTIME}::scene_fields::SceneValue, ::std::string::String> {{"
    ));
    for (id, class) in classes.iter() {
        if class.origin == ClassOrigin::Native {
            continue;
        }
        let own: Vec<&FieldInfo> = class
            .fields
            .iter()
            .filter(|field| field.is_exported)
            .collect();
        if own.is_empty() {
            continue;
        }
        out.open(&format!(
            "if let ::std::option::Option::Some({OBJECT}) = {}({RC}::clone(object_)) {{",
            context.cast_function(id)
        ));
        out.open(&format!("match {FIELD_ARGUMENT} {{"));
        for field in own {
            out.line(&format!(
                "{:?} => return ::std::result::Result::Ok({}),",
                field.name,
                field_value(context, id, field)
            ));
        }
        out.line("_ => {}");
        out.close("}");
        out.close("}");
    }
    out.line(&format!(
        "::std::result::Result::Err({RUNTIME}::scene_fields::no_property({FIELD_ARGUMENT}))"
    ));
    out.close("}");
}

/// The code that reads `field`, declared by `owner`, as a scene value.
fn field_value(context: Context<'_>, owner: ClassId, field: &FieldInfo) -> String {
    let classes = context.analysis.classes();
    let target = format!(
        "{}::HasFields::fields(&*{OBJECT}).{}",
        context.class_path(owner),
        ident(&field.name)
    );
    let value = format!("{RUNTIME}::scene_fields::SceneValue");
    match &field.ty {
        Type::Short | Type::Int | Type::BigInt => {
            format!("{value}::Int(::std::primitive::i64::from({target}.get()))")
        }
        Type::Float => format!("{value}::Float(::std::primitive::f64::from({target}.get()))"),
        Type::Double => format!("{value}::Float({target}.get())"),
        Type::Bool => format!("{value}::Bool({target}.get())"),
        Type::String => format!("{value}::Text({target}.get())"),
        Type::Class(id) if classes.class(*id).name == COLOR_CLASS => format!(
            "{value}::Color(<crate::classes::Host_ as {RUNTIME}::nodes::Host>::channels(&{target}.get()))"
        ),
        // Analysis lets only the types above be exported.
        _ => format!(
            "return ::std::result::Result::Err({RUNTIME}::scene_fields::no_property({FIELD_ARGUMENT}))"
        ),
    }
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
