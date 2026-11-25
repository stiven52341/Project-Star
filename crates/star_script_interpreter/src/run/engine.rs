//! Engine objects and methods (decision 70): the fields of colors and
//! nodes are the runtime's own, and `Input` reads the runtime's state, so
//! they behave as in the compiled game.

use std::rc::Rc;
use std::sync::Arc;

use star_script_runtime::Field;
use star_script_runtime::color::ColorFields;
use star_script_runtime::input::{action_state, axis, pointer};

use super::object_of;
use crate::error::RunError;
use crate::host::NodeParts;
use crate::tree::{ClassRef, EngineClass, EngineField, EngineMethod, FieldRef};
use crate::value::{Object, Value};

/// A field of an engine class missing from the object: lowering only
/// reads the fields of the object's own classes.
const MISSING_ENGINE_FIELD: RunError = RunError::Internal("engine field outside its object");

/// A value of a type the field does not have.
const WRONG_FIELD_VALUE: RunError =
    RunError::Internal("engine field given a value of another type");

/// The field `field` of `object`.
pub(super) fn read_field(object: &Value, field: FieldRef) -> Result<Value, RunError> {
    let field = match field {
        FieldRef::Script(position) => return object_of(object)?.field(position),
        FieldRef::Engine(field) => field,
    };
    if let Some(number) = float_field(object, field) {
        return Ok(Value::Float(number.get()));
    }
    match (object, field) {
        (Value::Node(node), EngineField::Name) => Ok(Value::String(
            node.engine.node.name.read(|name| Arc::from(name.as_str())),
        )),
        (Value::Node(node), EngineField::Main) => Ok(Value::Bool(camera_fields(node)?.main.get())),
        _ => Err(MISSING_ENGINE_FIELD),
    }
}

/// Puts `value` in the field `field` of `object`.
pub(super) fn write_field(object: &Value, field: FieldRef, value: Value) -> Result<(), RunError> {
    let field = match field {
        FieldRef::Script(position) => return object_of(object)?.set_field(position, value),
        FieldRef::Engine(field) => field,
    };
    if let Some(number) = float_field(object, field) {
        let Value::Float(value) = value else {
            return Err(WRONG_FIELD_VALUE);
        };
        number.set(value);
        return Ok(());
    }
    match (object, field, value) {
        (Value::Node(node), EngineField::Name, Value::String(name)) => {
            node.engine.node.name.set(name.to_string());
        }
        (Value::Node(node), EngineField::Main, Value::Bool(main)) => {
            camera_fields(node)?.main.set(main);
        }
        (Value::Node(_), EngineField::Name | EngineField::Main, _) => {
            return Err(WRONG_FIELD_VALUE);
        }
        _ => return Err(MISSING_ENGINE_FIELD),
    }
    Ok(())
}

/// The runtime's field `field` of `object`, if it is a `float` one.
fn float_field(object: &Value, field: EngineField) -> Option<&Field<f32>> {
    match (object, field) {
        (Value::Color(color), EngineField::Red) => Some(&color.engine.r),
        (Value::Color(color), EngineField::Green) => Some(&color.engine.g),
        (Value::Color(color), EngineField::Blue) => Some(&color.engine.b),
        (Value::Color(color), EngineField::Alpha) => Some(&color.engine.a),
        (Value::Node(node), EngineField::X) => Some(&node.engine.node.x),
        (Value::Node(node), EngineField::Y) => Some(&node.engine.node.y),
        (Value::Node(node), EngineField::Rotation) => Some(&node.engine.node.rotation),
        (Value::Node(node), EngineField::ScaleX) => Some(&node.engine.node.scale_x),
        (Value::Node(node), EngineField::ScaleY) => Some(&node.engine.node.scale_y),
        (Value::Node(node), EngineField::ViewHeight) => node
            .engine
            .camera_2d
            .as_deref()
            .map(|camera| &camera.view_height),
        _ => None,
    }
}

/// The fields of `Camera2D` of a node that is one.
fn camera_fields(
    node: &Object<NodeParts>,
) -> Result<&star_script_runtime::node_classes::Camera2dFields, RunError> {
    node.engine.camera_2d.as_deref().ok_or(MISSING_ENGINE_FIELD)
}

/// A new object of `class` whose engine class is `engine`: the runtime's
/// fields of `engine`, made from `arguments` (those of its constructor),
/// and the script fields `fields` of the classes that extend it.
pub(super) fn new_object(
    engine: EngineClass,
    class: ClassRef,
    fields: Vec<Value>,
    arguments: &[Value],
) -> Result<Value, RunError> {
    Ok(match engine {
        EngineClass::Color => {
            let [r, g, b, a] = arguments else {
                return Err(RunError::Internal("`Color` without its four channels"));
            };
            let channels = ColorFields::new(float(r)?, float(g)?, float(b)?, float(a)?);
            Value::Color(Rc::new(Object::new(class, fields, channels)))
        }
        EngineClass::Node | EngineClass::Camera2D => {
            let [Value::String(name)] = arguments else {
                return Err(RunError::Internal("a node without its name"));
            };
            let parts = NodeParts::new(engine, name.to_string());
            Value::Node(Rc::new(Object::new(class, fields, parts)))
        }
    })
}

/// Runs a static method of the engine.
pub(super) fn call(method: EngineMethod, arguments: &[Value]) -> Result<Value, RunError> {
    Ok(match (method, arguments) {
        (EngineMethod::IsDown, [action]) => Value::Bool(action_state(text(action)?).is_down),
        (EngineMethod::JustPressed, [action]) => {
            Value::Bool(action_state(text(action)?).just_pressed)
        }
        (EngineMethod::JustReleased, [action]) => {
            Value::Bool(action_state(text(action)?).just_released)
        }
        (EngineMethod::Strength, [action]) => Value::Float(action_state(text(action)?).strength),
        (EngineMethod::Axis, [negative, positive]) => {
            Value::Float(axis(text(negative)?, text(positive)?))
        }
        (EngineMethod::PointerX, []) => Value::Float(pointer().x),
        (EngineMethod::PointerY, []) => Value::Float(pointer().y),
        (EngineMethod::HasPointer, []) => Value::Bool(pointer().is_present),
        _ => return Err(RunError::Internal("engine method with other arguments")),
    })
}

fn float(value: &Value) -> Result<f32, RunError> {
    match value {
        Value::Float(value) => Ok(*value),
        _ => Err(RunError::Internal("a `float` argument that is not one")),
    }
}

fn text(value: &Value) -> Result<&str, RunError> {
    match value {
        Value::String(text) => Ok(text),
        _ => Err(RunError::Internal("a `string` argument that is not one")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(class: EngineClass) -> Value {
        new_object(class, class.class(), Vec::new(), &[Value::text("Seen")])
            .expect("a node of the engine")
    }

    #[test]
    fn engine_fields_are_the_runtime_s() {
        let camera = node(EngineClass::Camera2D);
        let view = FieldRef::Engine(EngineField::ViewHeight);

        write_field(&camera, view, Value::Float(5.0)).expect("a camera's field");

        assert_eq!(read_field(&camera, view).ok(), Some(Value::Float(5.0)));
        let Value::Node(object) = &camera else {
            panic!("a node");
        };
        assert!(
            object
                .engine
                .camera_2d
                .as_ref()
                .is_some_and(|fields| { (fields.view_height.get() - 5.0).abs() < f32::EPSILON })
        );
    }

    #[test]
    fn plain_nodes_have_no_camera_fields() {
        let plain = node(EngineClass::Node);

        let main = read_field(&plain, FieldRef::Engine(EngineField::Main));

        assert!(matches!(main, Err(RunError::Internal(_))));
    }

    #[test]
    fn actions_never_declared_are_never_held() {
        let held = call(EngineMethod::IsDown, &[Value::text("jump")]);
        let axis = call(
            EngineMethod::Axis,
            &[Value::text("left"), Value::text("right")],
        );

        assert_eq!(held.ok(), Some(Value::Bool(false)));
        assert_eq!(axis.ok(), Some(Value::Float(0.0)));
    }
}
