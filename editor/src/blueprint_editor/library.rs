//! The ready-made functions of the canvas's menu: the fields and methods
//! of the engine's classes and of numbers, as nodes already filled in.
//!
//! They are read from the classes StarScript knows (`native_classes`), so
//! what the engine adds shows up here without a second list to keep.

use std::sync::OnceLock;

use star_blueprint::{Call, CallTarget, NodeKind};
use star_script::semantic::{
    ClassId, ClassTable, FieldInfo, MethodInfo, NODE_3D_CLASS, NODE_CLASS, Type, builtin_methods,
    native_classes,
};

use crate::texts;

/// Methods the engine calls itself, which a blueprint redefines as graphs
/// rather than calls.
const EVENTS: [&str; 2] = ["start", "update"];
/// The members of nodes that place them, gathered under “Transform”.
const TRANSFORM_MEMBERS: [(&str, &str); 22] = [
    (NODE_CLASS, "x"),
    (NODE_CLASS, "y"),
    (NODE_CLASS, "rotation"),
    (NODE_CLASS, "scale_x"),
    (NODE_CLASS, "scale_y"),
    (NODE_CLASS, "global_x"),
    (NODE_CLASS, "global_y"),
    (NODE_CLASS, "global_rotation"),
    (NODE_CLASS, "set_global_position"),
    (NODE_CLASS, "set_global_rotation"),
    (NODE_CLASS, "translate"),
    (NODE_CLASS, "translate_global"),
    (NODE_CLASS, "rotate"),
    (NODE_3D_CLASS, "z"),
    (NODE_3D_CLASS, "pitch"),
    (NODE_3D_CLASS, "yaw"),
    (NODE_3D_CLASS, "scale_z"),
    (NODE_3D_CLASS, "look_at"),
    (NODE_3D_CLASS, "global_z"),
    (NODE_3D_CLASS, "set_global_position_3d"),
    (NODE_3D_CLASS, "translate_3d"),
    (NODE_3D_CLASS, "translate_global_3d"),
];
/// The name of the argument of the methods of numbers (`min`, `max`).
const NUMBER_ARGUMENT: &str = "other";

/// One entry of the menu: what it says and the node it adds.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct ReadyMade {
    pub(super) label: String,
    pub(super) kind: NodeKind,
}

/// The entries of one class, or of a theme such as “Transform”.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Group {
    pub(super) name: String,
    /// Members of the blueprint itself, and functions of the class.
    pub(super) own: Vec<ReadyMade>,
    /// Members of another object, given to the node's `target` pin.
    pub(super) of_other: Vec<ReadyMade>,
}

/// Every ready-made function a blueprint can add.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Library {
    groups: Vec<Group>,
}

impl Library {
    /// The functions for a blueprint that extends `base`. Members of a
    /// class are offered as the blueprint's own only if it extends that
    /// class; a base that is not the engine's (a class of the project)
    /// is taken to be a node.
    pub(super) fn new(base: Option<&str>) -> Self {
        let classes = engine_classes();
        let base = base.map(|name| classes.find(name).or_else(|| classes.find(NODE_CLASS)));
        let extends = |class: ClassId| {
            base.flatten()
                .is_some_and(|base| classes.is_subclass(base, class))
        };
        let mut groups = vec![transform_group(classes, &extends)];
        groups.extend(classes.iter().filter_map(|(id, _)| {
            let group = class_group(classes, id, extends(id));
            (!group.own.is_empty() || !group.of_other.is_empty()).then_some(group)
        }));
        groups.push(math_group());
        Self { groups }
    }

    pub(super) fn groups(&self) -> &[Group] {
        &self.groups
    }

    /// The entries whose label contains `text`, ignoring case, with the
    /// name of their group.
    pub(super) fn search(&self, text: &str) -> Vec<(&str, &ReadyMade)> {
        let text = text.to_lowercase();
        self.groups
            .iter()
            .flat_map(|group| {
                group
                    .own
                    .iter()
                    .chain(&group.of_other)
                    .map(move |entry| (group.name.as_str(), entry))
            })
            .filter(|(_, entry)| entry.label.to_lowercase().contains(&text))
            .collect()
    }
}

/// The engine's classes, read once.
fn engine_classes() -> &'static ClassTable {
    static CLASSES: OnceLock<ClassTable> = OnceLock::new();
    CLASSES.get_or_init(native_classes)
}

/// The members of nodes that place them.
fn transform_group(classes: &ClassTable, extends: &impl Fn(ClassId) -> bool) -> Group {
    let mut group = Group {
        name: texts::current().transform.to_owned(),
        own: Vec::new(),
        of_other: Vec::new(),
    };
    for (class_name, member) in TRANSFORM_MEMBERS {
        let Some(id) = classes.find(class_name) else {
            continue;
        };
        let class = classes.class(id);
        let add = |of_value: bool, into: &mut Vec<ReadyMade>| {
            if let Some(field) = class.fields.iter().find(|field| field.name == member) {
                into.extend(field_entries(classes, field, of_value));
            } else if let Some(method) = class.methods.iter().find(|method| method.name == member) {
                into.push(method_entry(classes, class_name, method, of_value));
            }
        };
        if extends(id) {
            add(false, &mut group.own);
        }
        add(true, &mut group.of_other);
    }
    group
}

/// The fields and methods of the class `id`: its functions and, if the
/// blueprint `is_one` of its objects, its own members; and the members of
/// another object of the class.
fn class_group(classes: &ClassTable, id: ClassId, is_one: bool) -> Group {
    let class = classes.class(id);
    let mut group = Group {
        name: class.name.clone(),
        own: Vec::new(),
        of_other: Vec::new(),
    };
    for field in class.fields.iter().filter(|field| !field.is_private) {
        if is_one {
            group.own.extend(field_entries(classes, field, false));
        }
        group.of_other.extend(field_entries(classes, field, true));
    }
    let callable = class.methods.iter().filter(|method| {
        !method.is_private && !method.is_async() && !EVENTS.contains(&method.name.as_str())
    });
    for method in callable {
        if method.is_static {
            group
                .own
                .push(method_entry(classes, &class.name, method, false));
        } else {
            if is_one {
                group
                    .own
                    .push(method_entry(classes, &class.name, method, false));
            }
            group
                .of_other
                .push(method_entry(classes, &class.name, method, true));
        }
    }
    group
}

/// The methods of a number, called on the value given to `target`.
fn math_group() -> Group {
    let of_other = builtin_methods(&Type::Float)
        .map(|signature| {
            let arguments = signature
                .parameters
                .iter()
                .map(|_| NUMBER_ARGUMENT.to_owned())
                .collect::<Vec<_>>();
            let call = Call {
                target: CallTarget::Value,
                method: signature.method.name().to_owned(),
                arguments,
                returns_value: true,
                pure: true,
            };
            ReadyMade {
                label: call_label(&call, &engine_classes().type_name(&signature.return_type)),
                kind: NodeKind::Call(call),
            }
        })
        .collect();
    Group {
        name: texts::current().math.to_owned(),
        own: Vec::new(),
        of_other,
    }
}

/// Reading and setting `field`, of the blueprint or, if `of_value`, of
/// another object.
fn field_entries(classes: &ClassTable, field: &FieldInfo, of_value: bool) -> [ReadyMade; 2] {
    let ty = classes.type_name(&field.ty);
    let get = NodeKind::GetVariable {
        variable: field.name.clone(),
        of_value,
    };
    let set = NodeKind::SetVariable {
        variable: field.name.clone(),
        of_value,
    };
    [get, set].map(|kind| ReadyMade {
        label: format!("{}: {ty}", texts::current().node_title(&kind)),
        kind,
    })
}

/// Calling `method` of the class `class`: as a function of the class if
/// it is static, else on the blueprint or, if `of_value`, on another
/// object. A call that gives a value only computes it (it is pure).
fn method_entry(
    classes: &ClassTable,
    class: &str,
    method: &MethodInfo,
    of_value: bool,
) -> ReadyMade {
    let target = if method.is_static {
        CallTarget::Class(class.to_owned())
    } else if of_value {
        CallTarget::Value
    } else {
        CallTarget::OwnObject
    };
    let returns_value = method.return_type != Type::Void;
    let call = Call {
        target,
        method: method.name.clone(),
        arguments: method
            .parameters
            .iter()
            .map(|parameter| parameter.name.clone())
            .collect(),
        returns_value,
        pure: returns_value,
    };
    let returned = if returns_value {
        classes.type_name(&method.return_type)
    } else {
        String::new()
    };
    ReadyMade {
        label: call_label(&call, &returned),
        kind: NodeKind::Call(call),
    }
}

/// `Input.is_down(action) -> bool`, as the call is written, with what it
/// gives if it gives something.
fn call_label(call: &Call, returned: &str) -> String {
    let prefix = match &call.target {
        CallTarget::OwnObject => String::new(),
        CallTarget::Value => ".".to_owned(),
        CallTarget::Class(class) => format!("{class}."),
    };
    let arguments = call.arguments.join(", ");
    let mut label = format!("{prefix}{}({arguments})", call.method);
    if !returned.is_empty() {
        label.push_str(" -> ");
        label.push_str(returned);
    }
    label
}

#[cfg(test)]
mod tests {
    use super::*;

    fn group<'library>(library: &'library Library, name: &str) -> &'library Group {
        library
            .groups()
            .iter()
            .find(|group| group.name == name)
            .unwrap()
    }

    fn labels(entries: &[ReadyMade]) -> Vec<&str> {
        entries.iter().map(|entry| entry.label.as_str()).collect()
    }

    #[test]
    fn nodes_get_their_own_transform_and_that_of_others() {
        let library = Library::new(Some("Sprite"));
        let transform = group(&library, texts::current().transform);

        assert!(labels(&transform.own).contains(&"x: float"));
        assert!(!labels(&transform.own).contains(&"z: float"));
        assert!(labels(&transform.of_other).contains(&".z: float"));
        assert!(labels(&transform.of_other).contains(&".look_at(x, y, z)"));
        assert!(labels(&transform.own).contains(&"translate(x, y)"));
        assert!(labels(&transform.own).contains(&"global_x() -> float"));
        assert!(!labels(&transform.own).contains(&"translate_3d(x, y, z)"));
    }

    #[test]
    fn nodes_can_be_found_by_name_and_by_tag() {
        let library = Library::new(Some("Node"));

        let scene = labels(&group(&library, "Scene").own);
        let node = labels(&group(&library, "Node").own);

        assert!(
            scene.contains(&"Scene.find_by_name(name) -> Node?"),
            "{scene:?}"
        );
        assert!(
            scene.contains(&"Scene.find_by_tag(tag) -> [Node]"),
            "{scene:?}"
        );
        assert!(node.contains(&"find_by_tag(tag) -> [Node]"), "{node:?}");
        assert!(node.contains(&"has_tag(tag) -> bool"), "{node:?}");
    }

    #[test]
    fn three_d_nodes_also_get_the_third_dimension() {
        let library = Library::new(Some("Mesh"));
        let own = labels(&group(&library, texts::current().transform).own);

        assert!(own.contains(&"z: float"));
        assert!(own.contains(&"look_at(x, y, z)"));
    }

    #[test]
    fn functions_of_a_class_are_offered_to_every_blueprint() {
        let library = Library::new(None);
        let input = group(&library, "Input");

        assert!(labels(&input.own).contains(&"Input.is_down(action) -> bool"));
        let NodeKind::Call(call) = &input.own[0].kind else {
            panic!("a call");
        };
        assert!(call.pure && call.returns_value);
        assert_eq!(group(&library, "Node").own, Vec::new());
    }

    #[test]
    fn events_and_waits_are_not_offered() {
        let library = Library::new(Some("Node"));

        assert_eq!(library.search("update(").len(), 0);
        assert_eq!(library.search("wait(").len(), 0);
        assert_eq!(library.search("find_child(").len(), 2);
    }

    #[test]
    fn blueprints_of_project_classes_are_taken_for_nodes() {
        let library = Library::new(Some("Enemy"));

        assert!(labels(&group(&library, "Node").own).contains(&"remove()"));
    }

    #[test]
    fn search_finds_entries_in_every_group() {
        let library = Library::new(Some("Node"));

        let found = library.search("IMPULSE");

        assert!(found.iter().all(|(group, _)| *group == "Body"));
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn numbers_offer_their_methods() {
        let library = Library::new(None);

        assert!(
            labels(&group(&library, texts::current().math).of_other)
                .contains(&".min(other) -> float")
        );
    }
}
