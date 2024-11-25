#![allow(dead_code)]

use std::{
    cell::{Ref, RefCell, RefMut},
    collections::BTreeMap,
    rc::Rc,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ContainerFormat {
    /// An empty struct, e.g. `struct A`.
    UnitStruct,
    /// A struct with a single unnamed parameter, e.g. `struct A(u16)`
    NewTypeStruct(Box<Format>),
    /// A struct with several unnamed parameters, e.g. `struct A(u16, u32)`
    TupleStruct(Vec<Format>),
    /// A struct with named parameters, e.g. `struct A { a: Foo }`.
    Struct(Vec<Named<Format>>),
    /// An enum, that is, an enumeration of variants.
    /// Each variant has a unique name and index within the enum.
    Enum(BTreeMap<u32, Named<VariantFormat>>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Format {
    /// A format whose value is initially unknown. Used internally for tracing. Not (de)serializable.
    Variable(Variable<Format>),
    /// The name of a container.
    TypeName(String),

    // The formats of primitive types
    Unit,
    Bool,
    I8,
    I16,
    I32,
    I64,
    I128,
    U8,
    U16,
    U32,
    U64,
    U128,
    F32,
    F64,
    Char,
    Str,
    Bytes,

    /// The format of `Option<T>`.
    Option(Box<Format>),
    /// A sequence, e.g. the format of `Vec<Foo>`.
    Seq(Box<Format>),
    /// A map, e.g. the format of `BTreeMap<K, V>`.
    Map {
        key: Box<Format>,
        value: Box<Format>,
    },

    /// A tuple, e.g. the format of `(Foo, Bar)`.
    Tuple(Vec<Format>),
    /// Alias for `(Foo, ... Foo)`.
    /// E.g. the format of `[Foo; N]`.
    TupleArray {
        content: Box<Format>,
        size: usize,
    },
}

/// A named value.
/// Used for named parameters or variants.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Named<T> {
    pub name: String,
    pub value: T,
}

/// A mutable holder for an initially unknown value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variable<T>(Rc<RefCell<Option<T>>>);

/// Description of a variant in an enum.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VariantFormat {
    /// A variant whose format is initially unknown. Used internally for tracing. Not (de)serializable.
    Variable(Variable<VariantFormat>),
    /// A variant without parameters, e.g. `A` in `enum X { A }`
    Unit,
    /// A variant with a single unnamed parameter, e.g. `A` in `enum X { A(u16) }`
    NewType(Box<Format>),
    /// A struct with several unnamed parameters, e.g. `A` in `enum X { A(u16, u32) }`
    Tuple(Vec<Format>),
    /// A struct with named parameters, e.g. `A` in `enum X { A { a: Foo } }`
    Struct(Vec<Named<Format>>),
}

impl Format {
    /// Return a format made of a fresh variable with no known value.
    pub fn unknown() -> Self {
        Self::Variable(Variable::new(None))
    }
}

impl VariantFormat {
    /// Return a format made of a fresh variable with no known value.
    pub fn unknown() -> Self {
        Self::Variable(Variable::new(None))
    }
}

impl Default for Format {
    fn default() -> Self {
        Self::unknown()
    }
}

impl Default for VariantFormat {
    fn default() -> Self {
        Self::unknown()
    }
}

impl<T> Variable<T> {
    pub(crate) fn new(content: Option<T>) -> Self {
        Self(Rc::new(RefCell::new(content)))
    }

    pub fn borrow(&self) -> Ref<Option<T>> {
        self.0.as_ref().borrow()
    }

    pub fn borrow_mut(&self) -> RefMut<Option<T>> {
        self.0.as_ref().borrow_mut()
    }
}

impl<T> Variable<T>
where
    T: Clone,
{
    fn into_inner(self) -> Option<T> {
        match Rc::try_unwrap(self.0) {
            Ok(cell) => cell.into_inner(),
            Err(rc) => rc.borrow().clone(),
        }
    }
}

impl<T: std::hash::Hash> std::hash::Hash for Variable<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.borrow().hash(state);
    }
}
