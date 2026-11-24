//! Every class a program can use, with its members.

use super::natives::COLOR_CLASS;
use super::{FieldDefault, InspectorHints, Type};
use crate::Span;

/// Identifies a class in a [`ClassTable`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClassId(usize);

impl ClassId {
    pub(super) const fn new(index: usize) -> Self {
        Self(index)
    }
}

/// Where a class comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassOrigin {
    /// Declared in the analyzed file, at this position of `SourceFile::classes`.
    Script {
        /// Index of the declaration.
        declaration: usize,
        /// Where its name is written.
        name: Span,
    },
    /// Provided by the engine and implemented in Rust.
    Native,
}

/// A class and its members.
#[derive(Debug, Clone, PartialEq)]
pub struct ClassInfo {
    /// Name of the class.
    pub name: String,
    /// Class it inherits from.
    pub base: Option<ClassId>,
    /// Where it comes from.
    pub origin: ClassOrigin,
    /// Fields declared by this class (not inherited ones).
    pub fields: Vec<FieldInfo>,
    /// Constants declared by this class.
    pub constants: Vec<ConstantInfo>,
    /// Methods declared by this class.
    pub methods: Vec<MethodInfo>,
    /// Its `init`, if it declares one.
    pub constructor: Option<ConstructorInfo>,
}

impl ClassInfo {
    pub(super) const fn new(name: String, origin: ClassOrigin) -> Self {
        Self {
            name,
            base: None,
            origin,
            fields: Vec::new(),
            constants: Vec::new(),
            methods: Vec::new(),
            constructor: None,
        }
    }

    /// The member declared by this class (not a base) with that name.
    #[must_use]
    pub fn own_member(&self, name: &str) -> Option<MemberRef> {
        let field = self.fields.iter().position(|field| field.name == name);
        let constant = self.constants.iter().position(|item| item.name == name);
        let method = self.methods.iter().position(|method| method.name == name);
        field
            .map(MemberRef::Field)
            .or(constant.map(MemberRef::Constant))
            .or(method.map(MemberRef::Method))
    }

    /// Parameters needed to create an object; none without an `init`.
    #[must_use]
    pub fn constructor_parameters(&self) -> &[ParameterInfo] {
        self.constructor
            .as_ref()
            .map_or(&[], |constructor| &constructor.parameters)
    }
}

/// `var name: Type` in a class.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldInfo {
    /// Name of the field.
    pub name: String,
    /// Its type, declared or inferred.
    pub ty: Type,
    /// Only usable inside its class.
    pub is_private: bool,
    /// `export`: scenes, and so the editor's inspector, can set it.
    pub is_exported: bool,
    /// Its first value, if the class writes it as a plain value.
    pub default: Option<FieldDefault>,
    /// How the inspector shows it, if it is `export`: its decorators.
    pub hints: InspectorHints,
    /// Whether it has `= value`.
    pub has_initializer: bool,
    /// Where its name is written.
    pub span: Span,
}

impl FieldInfo {
    /// Whether every `init` must assign it: an object field without a value.
    #[must_use]
    pub const fn needs_constructor(&self) -> bool {
        !self.has_initializer && !self.ty.has_default_value()
    }
}

/// `const NAME = value` in a class.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstantInfo {
    /// Name of the constant.
    pub name: String,
    /// Its type, declared or inferred.
    pub ty: Type,
    /// Only usable inside its class.
    pub is_private: bool,
    /// Where its name is written.
    pub span: Span,
    /// Whether its type is known yet; constants are resolved in source order.
    pub(super) resolved: bool,
}

/// `func name(parameters) -> Type` in a class.
#[derive(Debug, Clone, PartialEq)]
pub struct MethodInfo {
    /// Name of the method.
    pub name: String,
    /// Parameters, in order.
    pub parameters: Vec<ParameterInfo>,
    /// Type of the returned value, [`Type::Void`] if none.
    pub return_type: Type,
    /// Belongs to the class, not to each object.
    pub is_static: bool,
    /// Only usable inside its class.
    pub is_private: bool,
    /// Redefines a method of a base class.
    pub is_override: bool,
    /// How a call runs: at once, or as a coroutine.
    pub execution: Execution,
    /// Where its name is written; `None` for native methods.
    pub span: Option<Span>,
}

/// How a call of a method runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Execution {
    /// At once, until the method returns.
    Direct,
    /// `async`: as a coroutine, which can wait across frames with `await`.
    Async,
    /// `parallel`: on another thread, with copies of its arguments; awaited
    /// from coroutines, called directly from other `parallel` functions.
    Parallel,
}

impl MethodInfo {
    /// Whether the method is `async`.
    #[must_use]
    pub fn is_async(&self) -> bool {
        self.execution == Execution::Async
    }

    /// Whether the method is a `parallel` function.
    #[must_use]
    pub fn is_parallel(&self) -> bool {
        self.execution == Execution::Parallel
    }

    /// Whether `await` can wait for a call of the method.
    #[must_use]
    pub fn is_awaitable(&self) -> bool {
        self.execution != Execution::Direct
    }

    /// Whether `other` takes the same parameter types and returns the same type.
    #[must_use]
    pub fn has_same_signature(&self, other: &Self) -> bool {
        self.return_type == other.return_type
            && self.parameters.len() == other.parameters.len()
            && self
                .parameters
                .iter()
                .zip(&other.parameters)
                .all(|(mine, theirs)| mine.ty == theirs.ty)
    }
}

/// `init(parameters)` of a class.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstructorInfo {
    /// Parameters, in order.
    pub parameters: Vec<ParameterInfo>,
    /// Objects can only be created inside the class.
    pub is_private: bool,
    /// Where `init` is written.
    pub span: Span,
}

/// One parameter of a method or constructor.
#[derive(Debug, Clone, PartialEq)]
pub struct ParameterInfo {
    /// Name of the parameter.
    pub name: String,
    /// Its type.
    pub ty: Type,
}

/// A member, by its position in the lists of its class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberRef {
    /// Index into [`ClassInfo::fields`].
    Field(usize),
    /// Index into [`ClassInfo::constants`].
    Constant(usize),
    /// Index into [`ClassInfo::methods`].
    Method(usize),
}

/// A member found by name, and the class that declares it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FoundMember {
    /// The class that declares the member (maybe a base class).
    pub class: ClassId,
    /// Which member.
    pub member: MemberRef,
}

/// Every class known to a program: native ones first, then the script's.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ClassTable {
    classes: Vec<ClassInfo>,
}

impl ClassTable {
    pub(super) fn add(&mut self, class: ClassInfo) -> ClassId {
        self.classes.push(class);
        ClassId(self.classes.len() - 1)
    }

    /// The class with that identifier.
    #[must_use]
    pub fn class(&self, id: ClassId) -> &ClassInfo {
        // Identifiers are only created by `add`, so they are always in range.
        &self.classes[id.0]
    }

    pub(super) fn class_mut(&mut self, id: ClassId) -> &mut ClassInfo {
        &mut self.classes[id.0]
    }

    /// Every class, in the order they were added.
    pub fn iter(&self) -> impl Iterator<Item = (ClassId, &ClassInfo)> {
        self.classes
            .iter()
            .enumerate()
            .map(|(index, class)| (ClassId(index), class))
    }

    /// The class called `name`.
    #[must_use]
    pub fn find(&self, name: &str) -> Option<ClassId> {
        self.classes
            .iter()
            .position(|class| class.name == name)
            .map(ClassId)
    }

    /// `class`, its base, the base of its base, …
    ///
    /// Stops after visiting every class once, so even an inheritance cycle
    /// (reported elsewhere) cannot make it loop forever.
    pub fn ancestors(&self, class: ClassId) -> impl Iterator<Item = ClassId> + '_ {
        std::iter::successors(Some(class), |id| self.class(*id).base).take(self.classes.len())
    }

    /// Whether objects of `class` are also objects of `ancestor`.
    #[must_use]
    pub fn is_subclass(&self, class: ClassId, ancestor: ClassId) -> bool {
        self.ancestors(class).any(|id| id == ancestor)
    }

    /// The member called `name` in `class` or its closest base that has one.
    #[must_use]
    pub fn find_member(&self, class: ClassId, name: &str) -> Option<FoundMember> {
        self.ancestors(class).find_map(|id| {
            self.class(id)
                .own_member(name)
                .map(|member| FoundMember { class: id, member })
        })
    }

    /// Whether a value of type `from` can be stored where `to` is expected.
    #[must_use]
    pub fn is_assignable(&self, from: &Type, to: &Type) -> bool {
        match (from, to) {
            (Type::Error, _) | (_, Type::Error) | (Type::Null, Type::Nullable(_)) => true,
            (from, to) if from == to || from.widens_to(to) => true,
            (Type::Nullable(from), Type::Nullable(to)) => self.is_assignable(from, to),
            (from, Type::Nullable(to)) => self.is_assignable(from, to),
            (Type::Class(from), Type::Class(to)) => self.is_subclass(*from, *to),
            _ => false,
        }
    }

    /// Whether a field of type `ty` can be exported: a scene can write a
    /// number, a truth value, a text or a color.
    #[must_use]
    pub fn is_exportable(&self, ty: &Type) -> bool {
        match ty {
            Type::Short
            | Type::Int
            | Type::BigInt
            | Type::Float
            | Type::Double
            | Type::Bool
            | Type::String => true,
            Type::Class(id) => self.class(*id).name == COLOR_CLASS,
            _ => false,
        }
    }

    /// How the type is written in StarScript, for error messages.
    #[must_use]
    pub fn type_name(&self, ty: &Type) -> String {
        match ty {
            Type::Short => "short".to_owned(),
            Type::Int => "int".to_owned(),
            Type::BigInt => "bigint".to_owned(),
            Type::Float => "float".to_owned(),
            Type::Double => "double".to_owned(),
            Type::Bool => "bool".to_owned(),
            Type::String => "string".to_owned(),
            Type::Void => "nothing".to_owned(),
            Type::Null => "null".to_owned(),
            Type::Range => "range".to_owned(),
            Type::Class(id) => self.class(*id).name.clone(),
            Type::List(element) => format!("[{}]", self.type_name(element)),
            Type::Nullable(inner) => format!("{}?", self.type_name(inner)),
            Type::Error => "?".to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `Character` ← `Player`, and an unrelated `Rock`.
    fn hierarchy() -> (ClassTable, ClassId, ClassId, ClassId) {
        let mut table = ClassTable::default();
        let character = table.add(ClassInfo::new("Character".into(), ClassOrigin::Native));
        let player = table.add(ClassInfo::new("Player".into(), ClassOrigin::Native));
        let rock = table.add(ClassInfo::new("Rock".into(), ClassOrigin::Native));
        table.class_mut(player).base = Some(character);
        (table, character, player, rock)
    }

    #[test]
    fn subclasses_are_assignable_to_their_bases() {
        let (table, character, player, rock) = hierarchy();

        assert!(table.is_assignable(&Type::Class(player), &Type::Class(character)));
        assert!(!table.is_assignable(&Type::Class(character), &Type::Class(player)));
        assert!(!table.is_assignable(&Type::Class(rock), &Type::Class(character)));
    }

    #[test]
    fn null_only_fits_nullable_types() {
        let (table, character, ..) = hierarchy();

        assert!(table.is_assignable(&Type::Null, &Type::Class(character).nullable()));
        assert!(!table.is_assignable(&Type::Null, &Type::Class(character)));
        assert!(!table.is_assignable(&Type::Int.nullable(), &Type::Int));
        assert!(table.is_assignable(&Type::Int, &Type::Int.nullable()));
    }

    #[test]
    fn lists_must_match_exactly() {
        let (table, character, player, _) = hierarchy();

        assert!(!table.is_assignable(
            &Type::list(Type::Class(player)),
            &Type::list(Type::Class(character))
        ));
    }

    #[test]
    fn members_are_found_in_base_classes() {
        let (mut table, character, player, _) = hierarchy();
        table.class_mut(character).fields.push(FieldInfo {
            name: "x".into(),
            ty: Type::Float,
            is_private: false,
            is_exported: false,
            default: None,
            hints: InspectorHints::default(),
            has_initializer: true,
            span: Span::new(0, 0),
        });

        let found = table.find_member(player, "x");

        assert_eq!(
            found,
            Some(FoundMember {
                class: character,
                member: MemberRef::Field(0)
            })
        );
    }

    #[test]
    fn inheritance_cycles_do_not_loop_forever() {
        let (mut table, character, player, _) = hierarchy();
        table.class_mut(character).base = Some(player);

        assert_eq!(table.ancestors(player).count(), 3);
    }

    #[test]
    fn types_are_named_as_written() {
        let (table, _, player, _) = hierarchy();

        assert_eq!(
            table.type_name(&Type::list(Type::Class(player).nullable())),
            "[Player?]"
        );
    }
}
