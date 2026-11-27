//! Engine objects and methods (decisions 70 and 72): the fields of colors,
//! nodes, bodies and sounds are the runtime's own, the methods of `Node`
//! are the runtime's functions of nodes, those of `Body` and `Sound` the
//! runtime's methods of their fields, and `Input` and `Audio` read the
//! runtime's state, so they behave as in the compiled game.

use std::rc::Rc;
use std::sync::Arc;

use star_script_runtime::Task;
use star_script_runtime::audio::{self, SoundFields};
use star_script_runtime::bodies::{BodyFields, Contact};
use star_script_runtime::color::ColorFields;
use star_script_runtime::game::GameFields;
use star_script_runtime::input::{action_state, axis, pointer};
use star_script_runtime::node_classes::{
    AnimatorFields, BoneFields, Camera2dFields, Camera3dFields, LightFields, MeshFields,
    ModelFields, SpriteFields,
};
use star_script_runtime::nodes::{self, Host as _, Node3dFields};
use star_script_runtime::tweens::{self, Goal, Tween, TweenOptionsFields};
use star_script_runtime::{Field, List};
use star_script_runtime::{animations, machines, tracks};

use super::object_of;
use crate::error::RunError;
use crate::host::{
    BodyObject, CameraRef, ClassParts, GameObject, Interpreted, NodeObject, NodeParts, Tags,
};
use crate::play;
use crate::tree::{
    BodyMethod, BodyShape, ClassRef, EngineClass, EngineField, EngineMethod, FieldRef, NodeMethod,
    SoundMethod, TweenGoal,
};
use crate::value::{Object, Value};

/// A field of an engine class missing from the object: lowering only
/// reads the fields of the object's own classes.
const MISSING_ENGINE_FIELD: RunError = RunError::Internal("engine field outside its object");

/// A value of a type the field does not have.
const WRONG_FIELD_VALUE: RunError =
    RunError::Internal("engine field given a value of another type");

/// A runtime's field of an object, by the type of what it holds.
enum Slot<'a> {
    Float(&'a Field<f32>),
    Int(&'a Field<i32>),
    Bool(&'a Field<bool>),
    Text(&'a Field<String>),
    Color(&'a Field<Rc<Object<ColorFields>>>),
    Tags(&'a Field<Tags>),
    Camera(&'a Field<Option<CameraRef>>),
    Camera3D(&'a Field<Option<CameraRef<Camera3dFields>>>),
    Body(&'a Field<Option<Rc<BodyObject>>>),
}

/// The field `field` of `object`.
pub(super) fn read_field(object: &Value, field: FieldRef) -> Result<Value, RunError> {
    let field = match field {
        FieldRef::Script(position) => return object_of(object)?.field(position),
        FieldRef::Engine(field) => field,
    };
    Ok(match slot(object, field)? {
        Slot::Float(number) => Value::Float(number.get()),
        Slot::Int(number) => Value::Int(number.get()),
        Slot::Bool(flag) => Value::Bool(flag.get()),
        Slot::Text(text) => Value::String(text.read(|text| Arc::from(text.as_str()))),
        Slot::Color(color) => Value::Color(color.get()),
        Slot::Tags(tags) => Value::List(tags.get().0),
        Slot::Camera(camera) => camera.read(|camera| camera_value(camera.as_ref())),
        Slot::Camera3D(camera) => camera.read(|camera| camera_value(camera.as_ref())),
        Slot::Body(body) => body.get().map_or(Value::Null, Value::Body),
    })
}

/// Puts `value` in the field `field` of `object`.
pub(super) fn write_field(object: &Value, field: FieldRef, value: Value) -> Result<(), RunError> {
    let field = match field {
        FieldRef::Script(position) => return object_of(object)?.set_field(position, value),
        FieldRef::Engine(field) => field,
    };
    match (slot(object, field)?, value) {
        (Slot::Float(number), Value::Float(value)) => number.set(value),
        (Slot::Int(number), Value::Int(value)) => number.set(value),
        (Slot::Bool(flag), Value::Bool(value)) => flag.set(value),
        (Slot::Text(text), Value::String(value)) => text.set(value.to_string()),
        (Slot::Color(color), Value::Color(value)) => color.set(value),
        (Slot::Tags(tags), Value::List(elements)) => tags.set(Tags(elements)),
        (Slot::Camera(camera), Value::Null) => camera.set(None),
        (Slot::Camera(camera), Value::Node(node)) => {
            camera.set(Some(
                Interpreted::as_camera_2d(&node).ok_or(WRONG_FIELD_VALUE)?,
            ));
        }
        (Slot::Camera3D(camera), Value::Null) => camera.set(None),
        (Slot::Camera3D(camera), Value::Node(node)) => {
            camera.set(Some(
                Interpreted::as_camera_3d(&node).ok_or(WRONG_FIELD_VALUE)?,
            ));
        }
        (Slot::Body(body), Value::Null) => body.set(None),
        (Slot::Body(body), Value::Body(value)) => body.set(Some(value)),
        _ => return Err(WRONG_FIELD_VALUE),
    }
    Ok(())
}

/// The node of the camera a field of the game holds, or `null`.
fn camera_value<F>(camera: Option<&CameraRef<F>>) -> Value {
    camera.map_or(Value::Null, |camera| Value::Node(Rc::clone(camera.node())))
}

/// Where the runtime keeps the field `field` of `object`.
fn slot(object: &Value, field: EngineField) -> Result<Slot<'_>, RunError> {
    let slot = match object {
        Value::Color(color) => color_slot(&color.engine, field),
        Value::Node(node) => node_slot(&node.engine, field),
        Value::Game(game) => game_slot(&game.engine, field),
        Value::Body(body) => body_slot(&body.engine, field),
        Value::Sound(sound) => sound_slot(&sound.engine, field),
        Value::TweenOptions(options) => tween_options_slot(&options.engine, field),
        _ => None,
    };
    slot.ok_or(MISSING_ENGINE_FIELD)
}

fn color_slot(color: &ColorFields, field: EngineField) -> Option<Slot<'_>> {
    Some(Slot::Float(match field {
        EngineField::Red => &color.r,
        EngineField::Green => &color.g,
        EngineField::Blue => &color.b,
        EngineField::Alpha => &color.a,
        _ => return None,
    }))
}

fn node_slot(parts: &NodeParts, field: EngineField) -> Option<Slot<'_>> {
    let node = &parts.node;
    Some(match field {
        EngineField::Name => Slot::Text(&node.name),
        EngineField::X => Slot::Float(&node.x),
        EngineField::Y => Slot::Float(&node.y),
        EngineField::Rotation => Slot::Float(&node.rotation),
        EngineField::ScaleX => Slot::Float(&node.scale_x),
        EngineField::ScaleY => Slot::Float(&node.scale_y),
        EngineField::Tags => Slot::Tags(&node.tags),
        EngineField::Visible => Slot::Bool(&node.visible),
        EngineField::Body => Slot::Body(&node.body),
        _ => match (field, &parts.node_3d) {
            (
                EngineField::Z | EngineField::Pitch | EngineField::Yaw | EngineField::ScaleZ,
                Some(node_3d),
            ) => node_3d_slot(node_3d, field)?,
            _ => class_slot(&parts.class, field)?,
        },
    })
}

fn node_3d_slot(node: &Node3dFields, field: EngineField) -> Option<Slot<'_>> {
    Some(Slot::Float(match field {
        EngineField::Z => &node.z,
        EngineField::Pitch => &node.pitch,
        EngineField::Yaw => &node.yaw,
        EngineField::ScaleZ => &node.scale_z,
        _ => return None,
    }))
}

/// Where the runtime keeps the field `field` of the node class `class`.
fn class_slot(class: &ClassParts, field: EngineField) -> Option<Slot<'_>> {
    match class {
        ClassParts::Node => None,
        ClassParts::Camera2D(camera) => camera_slot(camera, field),
        ClassParts::Sprite(sprite) => sprite_slot(sprite, field),
        ClassParts::Light2D(light) | ClassParts::Light(light) => light_slot(light, field),
        ClassParts::Mesh(mesh) => mesh_slot(mesh, field),
        ClassParts::Camera3D(camera) => camera_3d_slot(camera, field),
        ClassParts::Animator(animator) => match field {
            EngineField::Animations => Some(Slot::Text(&animator.animations)),
            EngineField::States => Some(Slot::Text(&animator.states)),
            _ => None,
        },
        ClassParts::Model(model) => match field {
            EngineField::ModelFile => Some(Slot::Text(&model.file)),
            _ => None,
        },
        ClassParts::Bone2D(bone) => match field {
            EngineField::Length => Some(Slot::Float(&bone.length)),
            _ => None,
        },
    }
}

fn camera_slot(camera: &Camera2dFields, field: EngineField) -> Option<Slot<'_>> {
    Some(match field {
        EngineField::ViewHeight => Slot::Float(&camera.view_height),
        EngineField::Main => Slot::Bool(&camera.main),
        _ => return None,
    })
}

fn camera_3d_slot(camera: &Camera3dFields, field: EngineField) -> Option<Slot<'_>> {
    Some(match field {
        EngineField::Fov => Slot::Float(&camera.fov),
        EngineField::Main => Slot::Bool(&camera.main),
        _ => return None,
    })
}

fn mesh_slot(mesh: &MeshFields<Interpreted>, field: EngineField) -> Option<Slot<'_>> {
    Some(match field {
        EngineField::Shape => Slot::Text(&mesh.shape),
        EngineField::Width => Slot::Float(&mesh.width),
        EngineField::Height => Slot::Float(&mesh.height),
        EngineField::Depth => Slot::Float(&mesh.depth),
        EngineField::Radius => Slot::Float(&mesh.radius),
        EngineField::Tint => Slot::Color(&mesh.color),
        EngineField::Metallic => Slot::Float(&mesh.metallic),
        EngineField::Roughness => Slot::Float(&mesh.roughness),
        EngineField::Texture => Slot::Text(&mesh.texture),
        EngineField::Pixelated => Slot::Bool(&mesh.pixelated),
        _ => return None,
    })
}

fn sprite_slot(sprite: &SpriteFields<Interpreted>, field: EngineField) -> Option<Slot<'_>> {
    Some(match field {
        EngineField::Texture => Slot::Text(&sprite.texture),
        EngineField::Width => Slot::Float(&sprite.width),
        EngineField::Height => Slot::Float(&sprite.height),
        EngineField::Layer => Slot::Int(&sprite.layer),
        EngineField::Pixelated => Slot::Bool(&sprite.pixelated),
        EngineField::Tint => Slot::Color(&sprite.color),
        EngineField::Columns => Slot::Int(&sprite.columns),
        EngineField::Rows => Slot::Int(&sprite.rows),
        EngineField::Frame => Slot::Int(&sprite.frame),
        EngineField::Frames => Slot::Text(&sprite.frames),
        _ => return None,
    })
}

fn light_slot(light: &LightFields<Interpreted>, field: EngineField) -> Option<Slot<'_>> {
    Some(match field {
        EngineField::Kind => Slot::Text(&light.kind),
        EngineField::Tint => Slot::Color(&light.color),
        EngineField::Intensity => Slot::Float(&light.intensity),
        EngineField::Range => Slot::Float(&light.range),
        EngineField::Angle => Slot::Float(&light.angle),
        EngineField::Blend => Slot::Float(&light.blend),
        _ => return None,
    })
}

fn body_slot(body: &BodyFields<Interpreted>, field: EngineField) -> Option<Slot<'_>> {
    Some(match field {
        EngineField::Kind => Slot::Text(&body.kind),
        EngineField::Shape => Slot::Text(&body.shape),
        EngineField::Radius => Slot::Float(&body.radius),
        EngineField::Width => Slot::Float(&body.width),
        EngineField::Height => Slot::Float(&body.height),
        EngineField::Depth => Slot::Float(&body.depth),
        EngineField::OffsetX => Slot::Float(&body.offset_x),
        EngineField::OffsetY => Slot::Float(&body.offset_y),
        EngineField::OffsetZ => Slot::Float(&body.offset_z),
        EngineField::GravityScale => Slot::Float(&body.gravity_scale),
        EngineField::Friction => Slot::Float(&body.friction),
        EngineField::Bounciness => Slot::Float(&body.bounciness),
        EngineField::Density => Slot::Float(&body.density),
        EngineField::VelocityX => Slot::Float(&body.velocity_x),
        EngineField::VelocityY => Slot::Float(&body.velocity_y),
        EngineField::VelocityZ => Slot::Float(&body.velocity_z),
        EngineField::FixedRotation => Slot::Bool(&body.fixed_rotation),
        EngineField::Sensor => Slot::Bool(&body.sensor),
        _ => return None,
    })
}

fn tween_options_slot(options: &TweenOptionsFields, field: EngineField) -> Option<Slot<'_>> {
    Some(match field {
        EngineField::Delay => Slot::Float(&options.delay),
        EngineField::Back => Slot::Bool(&options.back),
        EngineField::Repeats => Slot::Int(&options.repeats),
        _ => return None,
    })
}

fn sound_slot(sound: &SoundFields, field: EngineField) -> Option<Slot<'_>> {
    Some(match field {
        EngineField::Volume => Slot::Float(&sound.volume),
        EngineField::Looping => Slot::Bool(&sound.looping),
        _ => return None,
    })
}

fn game_slot(game: &GameFields<Interpreted>, field: EngineField) -> Option<Slot<'_>> {
    Some(match field {
        EngineField::Background => Slot::Color(&game.background),
        EngineField::Camera => Slot::Camera(&game.camera),
        EngineField::Camera3D => Slot::Camera3D(&game.camera_3d),
        EngineField::Ambient => Slot::Color(&game.ambient),
        EngineField::Ambient2D => Slot::Color(&game.ambient_2d),
        _ => return None,
    })
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
        EngineClass::Game => {
            let root = game_root();
            play::set_game_root(&root);
            let game: GameObject = Object::new(class, fields, GameFields::new(root));
            Value::Game(Rc::new(game))
        }
        EngineClass::Node
        | EngineClass::Camera2D
        | EngineClass::Sprite
        | EngineClass::Light2D
        | EngineClass::Node3D
        | EngineClass::Mesh
        | EngineClass::Light
        | EngineClass::Camera3D
        | EngineClass::Animator
        | EngineClass::Bone2D
        | EngineClass::Model => Value::Node(Rc::new(Object::new(
            class,
            fields,
            node_parts(engine, arguments)?,
        ))),
        EngineClass::TweenOptions => Value::TweenOptions(Rc::new(Object::new(
            class,
            fields,
            TweenOptionsFields::new(),
        ))),
        EngineClass::Body | EngineClass::Sound => {
            // Their `init` is private: only their static methods make them.
            return Err(RunError::Internal("a body or sound built by a constructor"));
        }
    })
}

/// The runtime's fields of a new node of the node class `engine`, from
/// the arguments of its constructor.
fn node_parts(engine: EngineClass, arguments: &[Value]) -> Result<NodeParts, RunError> {
    let (name, parts) = match (engine, arguments) {
        (EngineClass::Node, [name]) => (name, ClassParts::Node),
        (EngineClass::Camera2D, [name]) => {
            (name, ClassParts::Camera2D(Rc::new(Camera2dFields::new())))
        }
        (EngineClass::Sprite, [name, texture]) => (
            name,
            ClassParts::Sprite(SpriteFields::new(text(texture)?.to_owned())),
        ),
        (EngineClass::Light2D, [name, kind]) => (
            name,
            ClassParts::Light2D(LightFields::new_2d(text(kind)?.to_owned())),
        ),
        (EngineClass::Node3D, [name]) => {
            return Ok(NodeParts::new_3d(text(name)?.to_owned(), ClassParts::Node));
        }
        (EngineClass::Mesh, [name, shape]) => {
            let mesh = ClassParts::Mesh(MeshFields::new(text(shape)?.to_owned()));
            return Ok(NodeParts::new_3d(text(name)?.to_owned(), mesh));
        }
        (EngineClass::Light, [name, kind]) => {
            let light = ClassParts::Light(LightFields::new_3d(text(kind)?.to_owned()));
            return Ok(NodeParts::new_3d(text(name)?.to_owned(), light));
        }
        (EngineClass::Camera3D, [name]) => {
            let camera = ClassParts::Camera3D(Rc::new(Camera3dFields::new()));
            return Ok(NodeParts::new_3d(text(name)?.to_owned(), camera));
        }
        (EngineClass::Animator, [name]) => (name, ClassParts::Animator(AnimatorFields::new())),
        (EngineClass::Bone2D, [name]) => (name, ClassParts::Bone2D(BoneFields::new())),
        (EngineClass::Model, [name, file]) => {
            let model = ClassParts::Model(ModelFields::new(text(file)?.to_owned()));
            return Ok(NodeParts::new_3d(text(name)?.to_owned(), model));
        }
        _ => return Err(RunError::Internal("a node without its arguments")),
    };
    Ok(NodeParts::new(text(name)?.to_owned(), parts))
}

/// The root of a game's tree: a `Node` called `Game`, as compiled.
fn game_root() -> Rc<NodeObject> {
    let name = EngineClass::Game.name().to_owned();
    Rc::new(Object::new(
        EngineClass::Node.class(),
        Vec::new(),
        NodeParts::new(name, ClassParts::Node),
    ))
}

/// Runs the engine's version of a method: a static one, or one of an
/// object (given first).
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
        (EngineMethod::Quit, [Value::Game(game)]) => {
            game.engine.quit();
            Value::Null
        }
        (EngineMethod::AddChild, [Value::Game(game), Value::Node(node)]) => {
            game.engine.add_child(Rc::clone(node));
            Value::Null
        }
        (EngineMethod::Start | EngineMethod::Update, _) => Value::Null,
        (EngineMethod::Node(method), [Value::Node(node), arguments @ ..]) => {
            node_method(method, node, arguments)?
        }
        (EngineMethod::FindInGame, [name]) => {
            let name = text(name)?;
            optional_node(
                play::game_root().and_then(|root| nodes::find_by_name::<Interpreted>(&root, name)),
            )
        }
        (EngineMethod::FindTaggedInGame, [tag]) => {
            let tag = text(tag)?;
            play::game_root().map_or_else(
                || Value::list(Vec::new()),
                |root| node_list(&nodes::find_by_tag::<Interpreted>(&root, tag)),
            )
        }
        (EngineMethod::NewBody(shape), [kind, sizes @ ..]) => new_body(shape, kind, sizes)?,
        (EngineMethod::Body(method), [Value::Body(body), arguments @ ..]) => {
            body_method(method, &body.engine, arguments)?
        }
        (EngineMethod::LoadSound, [path]) => {
            SoundFields::load(text(path)?).map_or(Value::Null, |fields| {
                Value::Sound(Rc::new(Object::new(
                    EngineClass::Sound.class(),
                    Vec::new(),
                    fields,
                )))
            })
        }
        (EngineMethod::Sound(method), [Value::Sound(sound)]) => sound_method(method, &sound.engine),
        (EngineMethod::Volume, []) => Value::Float(audio::volume()),
        (EngineMethod::SetVolume, [volume]) => {
            audio::set_volume(float(volume)?);
            Value::Null
        }
        _ => return Err(RunError::Internal("engine method with other arguments")),
    })
}

/// `Body.circle` and the like: a new body of `kind` with the sizes of
/// `shape`, as compiled.
fn new_body(shape: BodyShape, kind: &Value, sizes: &[Value]) -> Result<Value, RunError> {
    let kind = text(kind)?.to_owned();
    let fields = match (shape, sizes) {
        (BodyShape::Circle, [radius]) => BodyFields::circle(kind, float(radius)?),
        (BodyShape::Rectangle, [width, height]) => {
            BodyFields::rectangle(kind, float(width)?, float(height)?)
        }
        (BodyShape::Capsule, [height, radius]) => {
            BodyFields::capsule(kind, float(height)?, float(radius)?)
        }
        (BodyShape::Sphere, [radius]) => BodyFields::sphere(kind, float(radius)?),
        (BodyShape::Box, [width, height, depth]) => {
            BodyFields::cuboid(kind, float(width)?, float(height)?, float(depth)?)
        }
        (BodyShape::Mesh, []) => BodyFields::mesh(kind),
        _ => return Err(RunError::Internal("a body without its sizes")),
    };
    Ok(Value::Body(Interpreted::new_body(fields)))
}

/// Runs the method of `Body` `method` on the fields `body`.
fn body_method(
    method: BodyMethod,
    body: &BodyFields<Interpreted>,
    arguments: &[Value],
) -> Result<Value, RunError> {
    let contacts = |which| node_list(&body.contacts(which));
    Ok(match (method, arguments) {
        (BodyMethod::ApplyImpulse, [x, y]) => {
            body.apply_impulse(float(x)?, float(y)?, 0.0);
            Value::Null
        }
        (BodyMethod::ApplyImpulse3D, [x, y, z]) => {
            body.apply_impulse(float(x)?, float(y)?, float(z)?);
            Value::Null
        }
        (BodyMethod::Touching, []) => contacts(Contact::Touching),
        (BodyMethod::JustTouched, []) => contacts(Contact::JustTouched),
        (BodyMethod::JustSeparated, []) => contacts(Contact::JustSeparated),
        _ => return Err(RunError::Internal("method of `Body` with other arguments")),
    })
}

/// Runs the method of `Sound` `method` on the fields `sound`.
fn sound_method(method: SoundMethod, sound: &SoundFields) -> Value {
    match method {
        SoundMethod::Play => {
            sound.play();
            Value::Null
        }
        SoundMethod::Stop => {
            sound.stop();
            Value::Null
        }
        SoundMethod::IsPlaying => Value::Bool(sound.is_playing()),
    }
}

/// Runs the method of `Node` `method` on `node`, as compiled: with the
/// runtime's functions of nodes.
fn node_method(
    method: NodeMethod,
    node: &Rc<NodeObject>,
    arguments: &[Value],
) -> Result<Value, RunError> {
    type Runtime = Interpreted;
    Ok(match (method, arguments) {
        (NodeMethod::AddChild, [Value::Node(child)]) => {
            nodes::add_child::<Runtime>(node, Rc::clone(child));
            Value::Null
        }
        (NodeMethod::Remove, []) => {
            nodes::remove::<Runtime>(node);
            Value::Null
        }
        (NodeMethod::Parent, []) => optional_node(nodes::parent::<Runtime>(node)),
        (NodeMethod::Children, []) => node_list(&nodes::children::<Runtime>(node)),
        (NodeMethod::FindChild, [name]) => {
            optional_node(nodes::find_child::<Runtime>(node, text(name)?))
        }
        (NodeMethod::HasTag, [tag]) => Value::Bool(nodes::has_tag::<Runtime>(node, text(tag)?)),
        (NodeMethod::FindByName, [name]) => {
            optional_node(nodes::find_by_name::<Runtime>(node, text(name)?))
        }
        (NodeMethod::FindByTag, [tag]) => {
            node_list(&nodes::find_by_tag::<Runtime>(node, text(tag)?))
        }
        (NodeMethod::GlobalX, []) => Value::Float(nodes::global_position::<Runtime>(node)[0]),
        (NodeMethod::GlobalY, []) => Value::Float(nodes::global_position::<Runtime>(node)[1]),
        (NodeMethod::GlobalRotation, []) => Value::Float(nodes::global_rotation::<Runtime>(node)),
        (NodeMethod::SetGlobalPosition, [x, y]) => {
            nodes::set_global_position::<Runtime>(node, float(x)?, float(y)?);
            Value::Null
        }
        (NodeMethod::SetGlobalRotation, [angle]) => {
            nodes::set_global_rotation::<Runtime>(node, float(angle)?);
            Value::Null
        }
        (NodeMethod::Translate, [x, y]) => {
            nodes::translate::<Runtime>(node, float(x)?, float(y)?);
            Value::Null
        }
        (NodeMethod::TranslateGlobal, [x, y]) => {
            nodes::translate_global::<Runtime>(node, float(x)?, float(y)?);
            Value::Null
        }
        (NodeMethod::StopTweens, []) => {
            tweens::stop::<Runtime>(node);
            Value::Null
        }
        (NodeMethod::Rotate, [angle]) => {
            nodes::rotate::<Runtime>(node, float(angle)?);
            Value::Null
        }
        (NodeMethod::GlobalZ, []) => Value::Float(nodes::global_position::<Runtime>(node)[2]),
        (NodeMethod::LookAt, [x, y, z]) => {
            nodes::look_at::<Runtime>(node, float(x)?, float(y)?, float(z)?);
            Value::Null
        }
        (NodeMethod::SetGlobalPosition3D, [x, y, z]) => {
            nodes::set_global_position_3d::<Runtime>(node, [float(x)?, float(y)?, float(z)?]);
            Value::Null
        }
        (NodeMethod::Translate3D, [x, y, z]) => {
            nodes::translate_3d::<Runtime>(node, float(x)?, float(y)?, float(z)?);
            Value::Null
        }
        (NodeMethod::TranslateGlobal3D, [x, y, z]) => {
            nodes::translate_global_3d::<Runtime>(node, float(x)?, float(y)?, float(z)?);
            Value::Null
        }
        (NodeMethod::Play, [name]) => {
            animations::play::<Runtime>(node, text(name)?);
            Value::Null
        }
        (NodeMethod::Stop, []) => {
            animations::stop::<Runtime>(node);
            Value::Null
        }
        (NodeMethod::IsPlaying, []) => Value::Bool(animations::is_playing::<Runtime>(node)),
        // What a sprite or an animator does when an animation ends, unless
        // its script says.
        (NodeMethod::AnimationFinished, [_]) => Value::Null,
        (NodeMethod::Animation, []) => {
            Value::String(Arc::from(animations::animation::<Runtime>(node)))
        }
        (
            NodeMethod::AnimatorPlay
            | NodeMethod::AnimatorStop
            | NodeMethod::AnimatorIsPlaying
            | NodeMethod::AnimatorAnimation
            | NodeMethod::SetValue
            | NodeMethod::SetFlag
            | NodeMethod::Trigger
            | NodeMethod::State,
            _,
        ) => animator_method(method, node, arguments)?,
        _ => return Err(RunError::Internal("method of `Node` with other arguments")),
    })
}

/// A method of `Animator`: playing its animations and its state machine.
fn animator_method(
    method: NodeMethod,
    node: &Rc<NodeObject>,
    arguments: &[Value],
) -> Result<Value, RunError> {
    type Runtime = Interpreted;
    Ok(match (method, arguments) {
        (NodeMethod::AnimatorPlay, [name]) => {
            tracks::play::<Runtime>(node, text(name)?);
            Value::Null
        }
        (NodeMethod::AnimatorStop, []) => {
            tracks::stop::<Runtime>(node);
            Value::Null
        }
        (NodeMethod::AnimatorIsPlaying, []) => Value::Bool(tracks::is_playing::<Runtime>(node)),
        (NodeMethod::AnimatorAnimation, []) => {
            Value::String(Arc::from(tracks::animation::<Runtime>(node)))
        }
        (NodeMethod::SetValue, [name, value]) => {
            machines::set_value::<Runtime>(node, text(name)?, float(value)?);
            Value::Null
        }
        (NodeMethod::SetFlag, [name, value]) => {
            machines::set_flag::<Runtime>(node, text(name)?, flag(value)?);
            Value::Null
        }
        (NodeMethod::Trigger, [name]) => {
            machines::trigger::<Runtime>(node, text(name)?);
            Value::Null
        }
        (NodeMethod::State, []) => Value::String(Arc::from(machines::state::<Runtime>(node))),
        _ => {
            return Err(RunError::Internal(
                "method of `Animator` with other arguments",
            ));
        }
    })
}

/// A node the runtime found, or `null`.
fn optional_node(node: Option<Rc<NodeObject>>) -> Value {
    node.map_or(Value::Null, Value::Node)
}

/// A new list of the interpreter with the nodes of a list the runtime
/// made.
fn node_list(nodes: &List<Rc<NodeObject>>) -> Value {
    Value::list(nodes.to_vec().into_iter().map(Value::Node).collect())
}

/// The tween a tween method of `Node` starts (`goal` tells which), from
/// its arguments, the node first: it goes on while the node is in the
/// game's tree, as compiled.
///
/// # Errors
///
/// The runtime error of a tween that cannot be (see `tweens::start`).
pub(super) fn tween(goal: TweenGoal, arguments: &[Value]) -> Result<Task<()>, RunError> {
    let (node, property, to, seconds, ease, options) = match arguments {
        [Value::Node(node), property, to, seconds, ease] => {
            (node, property, to, seconds, ease, None)
        }
        [
            Value::Node(node),
            property,
            to,
            seconds,
            ease,
            Value::TweenOptions(options),
        ] => (node, property, to, seconds, ease, Some(options)),
        _ => return Err(RunError::Internal("a tween without its node and values")),
    };
    let target = match (goal, to) {
        (TweenGoal::To | TweenGoal::With, to) => Goal::To(float(to)?),
        (TweenGoal::By, by) => Goal::By(float(by)?),
        (TweenGoal::Color, Value::Color(color)) => Goal::Color(Interpreted::channels(color)),
        (TweenGoal::Color, _) => return Err(RunError::Internal("a color tween without a color")),
    };
    let mut started = Tween::new(
        text(property)?.to_owned(),
        target,
        float(seconds)?,
        text(ease)?.to_owned(),
    );
    if let Some(options) = options {
        started = started.with_options(&options.engine);
    }
    let owner = Rc::clone(node);
    Ok(tweens::start::<Interpreted>(node, started, move || {
        play::in_tree(&owner)
    })?)
}

fn float(value: &Value) -> Result<f32, RunError> {
    match value {
        Value::Float(value) => Ok(*value),
        _ => Err(RunError::Internal("a `float` argument that is not one")),
    }
}

fn flag(value: &Value) -> Result<bool, RunError> {
    match value {
        Value::Bool(value) => Ok(*value),
        _ => Err(RunError::Internal("a `bool` argument that is not one")),
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

    fn node(class: EngineClass, arguments: &[Value]) -> Value {
        new_object(class, class.class(), Vec::new(), arguments).expect("a node of the engine")
    }

    fn seen(class: EngineClass) -> Value {
        node(class, &[Value::text("Seen")])
    }

    #[test]
    fn engine_fields_are_the_runtime_s() {
        let camera = seen(EngineClass::Camera2D);
        let view = FieldRef::Engine(EngineField::ViewHeight);

        write_field(&camera, view, Value::Float(5.0)).expect("a camera's field");

        assert_eq!(read_field(&camera, view).ok(), Some(Value::Float(5.0)));
        let Value::Node(object) = &camera else {
            panic!("a node");
        };
        assert!(
            object
                .engine
                .camera_2d()
                .is_some_and(|fields| { (fields.view_height.get() - 5.0).abs() < f32::EPSILON })
        );
    }

    #[test]
    fn plain_nodes_have_no_camera_fields() {
        let plain = seen(EngineClass::Node);

        let main = read_field(&plain, FieldRef::Engine(EngineField::Main));

        assert!(matches!(main, Err(RunError::Internal(_))));
    }

    #[test]
    fn sprites_and_lights_start_as_compiled() {
        let sprite = node(
            EngineClass::Sprite,
            &[Value::text("Hero"), Value::text("hero.png")],
        );
        let light = node(
            EngineClass::Light2D,
            &[Value::text("Torch"), Value::text("spot")],
        );
        let read = |object: &Value, field| {
            read_field(object, FieldRef::Engine(field)).expect("a field of the node")
        };

        assert_eq!(read(&sprite, EngineField::Texture), Value::text("hero.png"));
        assert_eq!(read(&sprite, EngineField::Layer), Value::Int(0));
        assert_eq!(read(&light, EngineField::Kind), Value::text("spot"));
        assert_eq!(read(&light, EngineField::Range), Value::Float(3.0));
    }

    #[test]
    fn tags_written_by_scripts_are_seen_by_the_runtime() {
        let parent = seen(EngineClass::Node);
        let child = seen(EngineClass::Node);
        let tags = FieldRef::Engine(EngineField::Tags);
        call(
            EngineMethod::Node(NodeMethod::AddChild),
            &[parent.clone(), child.clone()],
        )
        .expect("a child added");

        let Value::List(elements) = read_field(&child, tags).expect("the child's tags") else {
            panic!("a list");
        };
        elements.borrow_mut().push(Value::text("enemy"));
        let found = call(
            EngineMethod::Node(NodeMethod::FindByTag),
            &[parent, Value::text("enemy")],
        )
        .expect("a search");

        assert_eq!(found, Value::list(vec![child]));
    }

    #[test]
    fn contacts_found_by_the_physics_are_new_lists_of_nodes() {
        let ball = seen(EngineClass::Node);
        let wall = seen(EngineClass::Node);
        let body = call(
            EngineMethod::NewBody(BodyShape::Circle),
            &[Value::text("dynamic"), Value::Float(0.25)],
        )
        .expect("a body");
        write_field(&ball, FieldRef::Engine(EngineField::Body), body.clone())
            .expect("the ball's body");
        let (Value::Body(fields), Value::Node(wall_node)) = (&body, &wall) else {
            panic!("a body and a node");
        };
        fields
            .engine
            .set_contacts(Contact::JustTouched, [wall_node]);

        let touched = call(
            EngineMethod::Body(BodyMethod::JustTouched),
            std::slice::from_ref(&body),
        );
        let held = read_field(&ball, FieldRef::Engine(EngineField::Body));

        assert_eq!(touched.ok(), Some(Value::list(vec![wall])));
        assert_eq!(held.ok(), Some(body));
    }

    #[test]
    fn impulses_add_up_until_the_physics_takes_them() {
        let body = call(
            EngineMethod::NewBody(BodyShape::Box),
            &[
                Value::text("dynamic"),
                Value::Float(1.0),
                Value::Float(2.0),
                Value::Float(3.0),
            ],
        )
        .expect("a body");
        let push = |arguments: &[Value]| {
            let mut all = vec![body.clone()];
            all.extend_from_slice(arguments);
            all
        };

        call(
            EngineMethod::Body(BodyMethod::ApplyImpulse),
            &push(&[Value::Float(1.0), Value::Float(2.0)]),
        )
        .expect("an impulse");
        call(
            EngineMethod::Body(BodyMethod::ApplyImpulse3D),
            &push(&[Value::Float(1.0), Value::Float(0.0), Value::Float(4.0)]),
        )
        .expect("an impulse");

        let Value::Body(fields) = &body else {
            panic!("a body");
        };
        assert_eq!(fields.engine.take_impulse(), [2.0, 2.0, 4.0]);
        assert_eq!(
            read_field(&body, FieldRef::Engine(EngineField::Shape)).ok(),
            Some(Value::text("box"))
        );
    }

    #[test]
    fn sounds_never_load_without_audio() {
        let sound = call(EngineMethod::LoadSound, &[Value::text("jump.wav")]);

        assert_eq!(sound.ok(), Some(Value::Null));
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
