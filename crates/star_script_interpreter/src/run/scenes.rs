//! Scenes (decision 71): `Scene.load` is the runtime's, which builds the
//! nodes through the interpreter's `Host`. The engine's node classes are
//! made as `Node(name)` and the like would make them, the program's as
//! `init()` does, and the values of their `export` fields are converted
//! with the runtime's `scene_fields`, so the reasons a scene is refused
//! are the compiled game's.

use std::rc::Rc;
use std::sync::Arc;

use star_script_runtime::nodes::Host as _;
use star_script_runtime::scene_fields::{self, SceneValue};
use star_script_runtime::scene_tree::{self, NewNode};

use super::{Machine, engine};
use crate::error::RunError;
use crate::host::{Interpreted, NodeObject};
use crate::play;
use crate::tree::{ConstructorRef, EngineClass, ExportKind, ExportedField};
use crate::value::Value;

/// Scenes are loaded only while the interpreter runs code.
const NO_PROGRAM: &str = "a scene loaded without the interpreter";

impl Machine {
    /// `Scene.load(path)`: the root of a new tree with the scene's nodes,
    /// or `null` if it cannot be loaded. A runtime error in the `init` of
    /// one of its classes stops the code that loads it, as compiled.
    pub(super) fn load_scene(&self, path: &str) -> Result<Value, RunError> {
        let root = play::loading_scenes(self, || scene_tree::load::<Interpreted>(path));
        if let Some(error) = self.take_failure() {
            return Err(error);
        }
        Ok(root.map_or(Value::Null, Value::Node))
    }

    /// A new object of the program's class `class` for a scene, as its
    /// `init()` leaves it.
    fn new_scene_object(&self, class: &str) -> Result<Rc<NodeObject>, String> {
        let constructor = self
            .code
            .scene_classes
            .get(class)
            .ok_or_else(|| format!("there is no class \"{class}\""))?
            .creation
            .clone()?;
        match self.construct(ConstructorRef::Script(constructor), Vec::new()) {
            Ok(Value::Node(node)) => Ok(node),
            Ok(_) => {
                Err(RunError::Internal("a scene class whose objects are not nodes").to_string())
            }
            Err(error) => {
                let reason = error.to_string();
                // `load_scene` stops the code that loaded the scene with it.
                self.fail(error);
                Err(reason)
            }
        }
    }

    /// Gives the `export` fields of `node`, an object of `class`, the
    /// values of its scene.
    fn set_exported(
        &self,
        class: &str,
        node: &Rc<NodeObject>,
        values: &[(String, SceneValue)],
    ) -> Result<(), String> {
        let exported = self
            .code
            .scene_classes
            .get(class)
            .map_or(&[][..], |class| class.exported.as_slice());
        for (name, value) in values {
            let field = exported
                .iter()
                .find(|field| field.name == *name)
                .ok_or_else(|| scene_fields::unknown_field(class, name))?;
            node.set_field(field.position, scene_value(field, value)?)
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }
}

impl Machine {
    /// Gives the `export` field `name` of `node`, an object of any class of
    /// the program, `value`, as an animation does.
    fn set_exported_field(
        &self,
        node: &Rc<NodeObject>,
        name: &str,
        value: &SceneValue,
    ) -> Result<(), String> {
        let field = self
            .code
            .classes
            .get(node.class().0)
            .and_then(|class| self.code.scene_classes.get(&*class.name))
            .and_then(|class| class.exported.iter().find(|field| field.name == name))
            .ok_or_else(|| scene_fields::no_property(name))?;
        node.set_field(field.position, scene_value(field, value)?)
            .map_err(|error| error.to_string())
    }

    /// The value of the `export` field `name` of `node`, an object of any
    /// class of the program, as a tween reads it.
    fn exported_field(&self, node: &Rc<NodeObject>, name: &str) -> Result<SceneValue, String> {
        let field = self
            .code
            .classes
            .get(node.class().0)
            .and_then(|class| self.code.scene_classes.get(&*class.name))
            .and_then(|class| class.exported.iter().find(|field| field.name == name))
            .ok_or_else(|| scene_fields::no_property(name))?;
        let value = node
            .field(field.position)
            .map_err(|error| error.to_string())?;
        Ok(match value {
            Value::Short(number) => SceneValue::Int(i64::from(number)),
            Value::Int(number) => SceneValue::Int(i64::from(number)),
            Value::BigInt(number) => SceneValue::Int(number),
            Value::Float(number) => SceneValue::Float(f64::from(number)),
            Value::Double(number) => SceneValue::Float(number),
            Value::Bool(truth) => SceneValue::Bool(truth),
            Value::String(text) => SceneValue::Text(text.to_string()),
            Value::Color(color) => SceneValue::Color(Interpreted::channels(&color)),
            _ => return Err(scene_fields::no_property(name)),
        })
    }
}

/// `Host::exported_field`: the value of the `export` field `name` of
/// `node`, as a tween reads it.
///
/// # Errors
///
/// Why it cannot be read: the class of `node` has no such `export` field.
pub(crate) fn exported_field(node: &Rc<NodeObject>, name: &str) -> Result<SceneValue, String> {
    play::machine()
        .or_else(play::scene_program)
        .ok_or_else(|| NO_PROGRAM.to_owned())?
        .exported_field(node, name)
}

/// `Host::set_exported_field`: gives the `export` field `name` of `node`
/// the value an animation gives it.
///
/// # Errors
///
/// Why it cannot be given: the class of `node` has no such `export` field,
/// or the value does not fit it.
pub(crate) fn set_exported_field(
    node: &Rc<NodeObject>,
    name: &str,
    value: &SceneValue,
) -> Result<(), String> {
    play::machine()
        .or_else(play::scene_program)
        .ok_or_else(|| NO_PROGRAM.to_owned())?
        .set_exported_field(node, name, value)
}

/// `Host::create`: a new node of `class`, called `name`, for a scene.
///
/// # Errors
///
/// Why the node cannot be created: the class cannot be used by a scene,
/// or its `init` failed.
pub(crate) fn create(name: String, class: NewNode<'_>) -> Result<Rc<NodeObject>, String> {
    let (engine, arguments) = match class {
        NewNode::Node => (EngineClass::Node, Vec::new()),
        NewNode::Camera2D => (EngineClass::Camera2D, Vec::new()),
        NewNode::Sprite { texture } => (EngineClass::Sprite, vec![text(texture)]),
        NewNode::Light2D { kind } => (EngineClass::Light2D, vec![text(kind)]),
        NewNode::Node3D => (EngineClass::Node3D, Vec::new()),
        NewNode::Mesh { shape } => (EngineClass::Mesh, vec![text(shape)]),
        NewNode::Light { kind } => (EngineClass::Light, vec![text(kind)]),
        NewNode::Camera3D => (EngineClass::Camera3D, Vec::new()),
        NewNode::Animator => (EngineClass::Animator, Vec::new()),
        NewNode::Bone2D => (EngineClass::Bone2D, Vec::new()),
        NewNode::Model { file } => (EngineClass::Model, vec![text(file)]),
        NewNode::SoundPlayer { sound } => (EngineClass::SoundPlayer, vec![text(sound)]),
        NewNode::Script(class) => {
            return play::scene_program()
                .ok_or_else(|| NO_PROGRAM.to_owned())?
                .new_scene_object(class);
        }
    };
    let arguments: Vec<Value> = std::iter::once(Value::String(Arc::from(name)))
        .chain(arguments)
        .collect();
    match engine::new_object(engine, engine.class(), Vec::new(), &arguments) {
        Ok(Value::Node(node)) => Ok(node),
        Ok(_) => {
            Err(RunError::Internal("an engine node class whose objects are not nodes").to_string())
        }
        Err(error) => Err(error.to_string()),
    }
}

/// `Host::set_exported`: gives the `export` fields of `node` the values
/// its scene writes.
///
/// # Errors
///
/// Why a value cannot be given: `class` has no such `export` field, or the
/// value does not fit it.
pub(crate) fn set_exported(
    class: &str,
    node: &Rc<NodeObject>,
    values: &[(String, SceneValue)],
) -> Result<(), String> {
    play::scene_program()
        .ok_or_else(|| NO_PROGRAM.to_owned())?
        .set_exported(class, node, values)
}

/// The value of a scene for `field`, converted to its type.
fn scene_value(field: &ExportedField, value: &SceneValue) -> Result<Value, String> {
    let name = field.name.as_str();
    Ok(match field.kind {
        ExportKind::Short => Value::Short(scene_fields::short(name, value)?),
        ExportKind::Int => Value::Int(scene_fields::int(name, value)?),
        ExportKind::BigInt => Value::BigInt(scene_fields::bigint(name, value)?),
        ExportKind::Float => Value::Float(scene_fields::float(name, value)?),
        ExportKind::Double => Value::Double(scene_fields::double(name, value)?),
        ExportKind::Bool => Value::Bool(scene_fields::boolean(name, value)?),
        ExportKind::String => Value::String(Arc::from(scene_fields::text(name, value)?)),
        ExportKind::Color => {
            let [r, g, b, a] = scene_fields::color(name, value)?;
            Value::Color(Interpreted::color(r, g, b, a))
        }
    })
}

fn text(text: &str) -> Value {
    Value::String(Arc::from(text))
}
