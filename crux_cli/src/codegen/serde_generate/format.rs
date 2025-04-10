// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Module defining the Abstract Syntax Tree (AST) of Serde formats.
//!
//! Node of the AST are made of the following types:
//! * `ContainerFormat`: the format of a container (struct or enum),
//! * `Format`: the format of an unnamed value,
//! * `Named<Format>`: the format of a field in a struct,
//! * `VariantFormat`: the format of a variant in a enum,
//! * `Named<VariantFormat>`: the format of a variant in a enum, together with its name,
//! * `Variable<Format>`: a variable holding an initially unknown value format,
//! * `Variable<VariantFormat>`: a variable holding an initially unknown variant format.

#![allow(dead_code)]

use error::{Error, Result};
use serde::{
    de, ser,
    ser::{SerializeMap, SerializeStruct},
    Deserialize, Serialize,
};
use std::{
    cell::{Ref, RefCell, RefMut},
    collections::{btree_map::Entry, BTreeMap},
    ops::DerefMut,
    rc::Rc,
};

use super::error;

/// Serde-based serialization format for anonymous "value" types.
#[derive(Serialize, Deserialize, Debug, Eq, Clone, PartialEq, Hash)]
#[serde(rename_all = "UPPERCASE")]
pub enum Format {
    /// A format whose value is initially unknown. Used internally for tracing. Not (de)serializable.
    Variable(#[serde(with = "not_implemented")] Variable<Format>),
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
    #[serde(rename_all = "UPPERCASE")]
    Map {
        key: Box<Format>,
        value: Box<Format>,
    },

    /// A tuple, e.g. the format of `(Foo, Bar)`.
    Tuple(Vec<Format>),
    /// Alias for `(Foo, ... Foo)`.
    /// E.g. the format of `[Foo; N]`.
    #[serde(rename_all = "UPPERCASE")]
    TupleArray {
        content: Box<Format>,
        size: usize,
    },
}

/// Serde-based serialization format for named "container" types.
/// In Rust, those are enums and structs.
#[derive(Serialize, Deserialize, Debug, Eq, Clone, PartialEq, Hash)]
#[serde(rename_all = "UPPERCASE")]
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

#[derive(Debug, Clone, Default, Eq, PartialEq, Hash)]
/// A named value.
/// Used for named parameters or variants.
pub struct Named<T> {
    pub name: String,
    pub value: T,
}

#[derive(Debug, Clone, Default, Eq, PartialEq)]
/// A mutable holder for an initially unknown value.
pub struct Variable<T>(Rc<RefCell<Option<T>>>);

impl<T: std::hash::Hash> std::hash::Hash for Variable<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.borrow().hash(state);
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
#[serde(rename_all = "UPPERCASE")]
/// Description of a variant in an enum.
pub enum VariantFormat {
    /// A variant whose format is initially unknown. Used internally for tracing. Not (de)serializable.
    Variable(#[serde(with = "not_implemented")] Variable<VariantFormat>),
    /// A variant without parameters, e.g. `A` in `enum X { A }`
    Unit,
    /// A variant with a single unnamed parameter, e.g. `A` in `enum X { A(u16) }`
    NewType(Box<Format>),
    /// A struct with several unnamed parameters, e.g. `A` in `enum X { A(u16, u32) }`
    Tuple(Vec<Format>),
    /// A struct with named parameters, e.g. `A` in `enum X { A { a: Foo } }`
    Struct(Vec<Named<Format>>),
}

/// Common methods for nodes in the AST of formats.
pub trait FormatHolder {
    /// Visit all the formats in `self` in a depth-first way.
    /// Variables are not supported and will cause an error.
    fn visit<'a>(&'a self, f: &mut dyn FnMut(&'a Format) -> Result<()>) -> Result<()>;

    /// Mutably visit all the formats in `self` in a depth-first way.
    /// * Replace variables (if any) with their known values then apply the
    ///   visiting function `f`.
    /// * Return an error if any variable has still an unknown value (thus cannot be removed).
    fn visit_mut(&mut self, f: &mut dyn FnMut(&mut Format) -> Result<()>) -> Result<()>;

    /// Update variables and add missing enum variants so that the terms match.
    /// This is a special case of [term unification](https://en.wikipedia.org/wiki/Unification_(computer_science)):
    /// * Variables occurring in `other` must be "fresh" and distinct
    ///   from each other. By "fresh", we mean that they do not occur in `self`
    ///   and have no known value yet.
    /// * If needed, enums in `self` will be extended with new variants taken from `other`.
    /// * Although the parameter `other` is consumed (i.e. taken by value), all
    ///   variables occurring either in `self` or `other` are correctly updated.
    fn unify(&mut self, other: Self) -> Result<()>;

    /// Finalize the formats within `self` by removing variables and making sure
    /// that all eligible tuples are compressed into a `TupleArray`. Return an error
    /// if any variable has an unknown value.
    fn normalize(&mut self) -> Result<()> {
        self.visit_mut(&mut |format: &mut Format| {
            let normalized = match format {
                Format::Tuple(formats) => {
                    let size = formats.len();
                    if size <= 1 {
                        return Ok(());
                    }
                    let format0 = &formats[0];
                    for format in formats.iter().skip(1) {
                        if format != format0 {
                            return Ok(());
                        }
                    }
                    Format::TupleArray {
                        content: Box::new(std::mem::take(&mut formats[0])),
                        size,
                    }
                }
                _ => {
                    return Ok(());
                }
            };
            *format = normalized;
            Ok(())
        })
    }

    /// Attempt to remove known variables within `self`. Silently abort
    /// if some variables have unknown values.
    fn reduce(&mut self) {
        self.visit_mut(&mut |_| Ok(())).unwrap_or(())
    }

    /// Whether this format is a variable with no known value yet.
    fn is_unknown(&self) -> bool;
}

fn unification_error<T1, T2>(v1: T1, v2: T2) -> Error
where
    T1: std::fmt::Debug,
    T2: std::fmt::Debug,
{
    Error::Incompatible(format!("{:?}", v1), format!("{:?}", v2))
}

impl FormatHolder for VariantFormat {
    fn visit<'a>(&'a self, f: &mut dyn FnMut(&'a Format) -> Result<()>) -> Result<()> {
        match self {
            Self::Variable(variable) => variable.visit(f)?,
            Self::Unit => (),
            Self::NewType(format) => format.visit(f)?,
            Self::Tuple(formats) => {
                for format in formats {
                    format.visit(f)?;
                }
            }
            Self::Struct(named_formats) => {
                for format in named_formats {
                    format.visit(f)?;
                }
            }
        }
        Ok(())
    }

    fn visit_mut(&mut self, f: &mut dyn FnMut(&mut Format) -> Result<()>) -> Result<()> {
        match self {
            Self::Variable(variable) => {
                variable.visit_mut(f)?;
                // At this point, `variable` is known and points to variable-free content.
                // Remove the variable.
                *self = std::mem::take(variable)
                    .into_inner()
                    .expect("variable is known");
            }
            Self::Unit => (),
            Self::NewType(format) => {
                format.visit_mut(f)?;
            }
            Self::Tuple(formats) => {
                for format in formats {
                    format.visit_mut(f)?;
                }
            }
            Self::Struct(named_formats) => {
                for format in named_formats {
                    format.visit_mut(f)?;
                }
            }
        }
        Ok(())
    }

    fn unify(&mut self, format: VariantFormat) -> Result<()> {
        match (self, format) {
            (format1, Self::Variable(variable2)) => {
                if let Some(format2) = variable2.borrow_mut().deref_mut() {
                    format1.unify(std::mem::take(format2))?;
                }
                *variable2.borrow_mut() = Some(format1.clone());
            }
            (Self::Variable(variable1), format2) => {
                let inner_variable = match variable1.borrow_mut().deref_mut() {
                    value1 @ None => {
                        *value1 = Some(format2);
                        None
                    }
                    Some(format1) => {
                        format1.unify(format2)?;
                        match format1 {
                            Self::Variable(variable) => Some(variable.clone()),
                            _ => None,
                        }
                    }
                };
                // Reduce multiple indirections to a single one.
                if let Some(variable) = inner_variable {
                    *variable1 = variable;
                }
            }

            (Self::Unit, Self::Unit) => (),

            (Self::NewType(format1), Self::NewType(format2)) => {
                format1.as_mut().unify(*format2)?;
            }

            (Self::Tuple(formats1), Self::Tuple(formats2)) if formats1.len() == formats2.len() => {
                for (format1, format2) in formats1.iter_mut().zip(formats2.into_iter()) {
                    format1.unify(format2)?;
                }
            }

            (Self::Struct(named_formats1), Self::Struct(named_formats2))
                if named_formats1.len() == named_formats2.len() =>
            {
                for (format1, format2) in named_formats1.iter_mut().zip(named_formats2.into_iter())
                {
                    format1.unify(format2)?;
                }
            }

            (format1, format2) => {
                return Err(unification_error(format1, format2));
            }
        }
        Ok(())
    }

    fn is_unknown(&self) -> bool {
        if let Self::Variable(v) = self {
            return v.is_unknown();
        }
        false
    }
}

impl<T> FormatHolder for Named<T>
where
    T: FormatHolder + std::fmt::Debug,
{
    fn visit<'a>(&'a self, f: &mut dyn FnMut(&'a Format) -> Result<()>) -> Result<()> {
        self.value.visit(f)
    }

    fn visit_mut(&mut self, f: &mut dyn FnMut(&mut Format) -> Result<()>) -> Result<()> {
        self.value.visit_mut(f)
    }

    fn unify(&mut self, other: Named<T>) -> Result<()> {
        if self.name != other.name {
            return Err(unification_error(&*self, &other));
        }
        self.value.unify(other.value)
    }

    fn is_unknown(&self) -> bool {
        false
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

mod not_implemented {
    pub fn serialize<T, S>(_: &T, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        use serde::ser::Error;
        Err(S::Error::custom("Cannot serialize variables"))
    }

    pub fn deserialize<'de, T, D>(_deserializer: D) -> Result<T, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        use serde::de::Error;
        Err(D::Error::custom("Cannot deserialize variables"))
    }
}

impl<T> FormatHolder for Variable<T>
where
    T: FormatHolder + std::fmt::Debug + Clone,
{
    fn visit<'a>(&'a self, _f: &mut dyn FnMut(&'a Format) -> Result<()>) -> Result<()> {
        Err(Error::NotSupported(
            "Cannot immutability visit formats with variables",
        ))
    }

    fn visit_mut(&mut self, f: &mut dyn FnMut(&mut Format) -> Result<()>) -> Result<()> {
        match self.borrow_mut().deref_mut() {
            None => Err(Error::UnknownFormat),
            Some(value) => value.visit_mut(f),
        }
    }

    fn unify(&mut self, _other: Variable<T>) -> Result<()> {
        // Omitting this method because a correct implementation would require
        // additional assumptions on T (in order to create new variables of type `T`).
        Err(Error::NotSupported("Cannot unify variables directly"))
    }

    fn is_unknown(&self) -> bool {
        match self.borrow().as_ref() {
            None => true,
            Some(format) => format.is_unknown(),
        }
    }
}

impl FormatHolder for ContainerFormat {
    fn visit<'a>(&'a self, f: &mut dyn FnMut(&'a Format) -> Result<()>) -> Result<()> {
        match self {
            Self::UnitStruct => (),
            Self::NewTypeStruct(format) => format.visit(f)?,
            Self::TupleStruct(formats) => {
                for format in formats {
                    format.visit(f)?;
                }
            }
            Self::Struct(named_formats) => {
                for format in named_formats {
                    format.visit(f)?;
                }
            }
            Self::Enum(variants) => {
                for variant in variants {
                    variant.1.visit(f)?;
                }
            }
        }
        Ok(())
    }

    fn visit_mut(&mut self, f: &mut dyn FnMut(&mut Format) -> Result<()>) -> Result<()> {
        match self {
            Self::UnitStruct => (),
            Self::NewTypeStruct(format) => format.visit_mut(f)?,
            Self::TupleStruct(formats) => {
                for format in formats {
                    format.visit_mut(f)?;
                }
            }
            Self::Struct(named_formats) => {
                for format in named_formats {
                    format.visit_mut(f)?;
                }
            }
            Self::Enum(variants) => {
                for variant in variants {
                    variant.1.visit_mut(f)?;
                }
            }
        }
        Ok(())
    }

    fn unify(&mut self, format: ContainerFormat) -> Result<()> {
        match (self, format) {
            (Self::UnitStruct, Self::UnitStruct) => (),

            (Self::NewTypeStruct(format1), Self::NewTypeStruct(format2)) => {
                format1.as_mut().unify(*format2)?;
            }

            (Self::TupleStruct(formats1), Self::TupleStruct(formats2))
                if formats1.len() == formats2.len() =>
            {
                for (format1, format2) in formats1.iter_mut().zip(formats2.into_iter()) {
                    format1.unify(format2)?;
                }
            }

            (Self::Struct(named_formats1), Self::Struct(named_formats2))
                if named_formats1.len() == named_formats2.len() =>
            {
                for (format1, format2) in named_formats1.iter_mut().zip(named_formats2.into_iter())
                {
                    format1.unify(format2)?;
                }
            }

            (Self::Enum(variants1), Self::Enum(variants2)) => {
                for (index2, variant2) in variants2.into_iter() {
                    match variants1.entry(index2) {
                        Entry::Vacant(e) => {
                            // Note that we do not check for name collisions.
                            e.insert(variant2);
                        }
                        Entry::Occupied(mut e) => {
                            e.get_mut().unify(variant2)?;
                        }
                    }
                }
            }

            (format1, format2) => {
                return Err(unification_error(format1, format2));
            }
        }
        Ok(())
    }

    fn is_unknown(&self) -> bool {
        false
    }
}

impl FormatHolder for Format {
    fn visit<'a>(&'a self, f: &mut dyn FnMut(&'a Format) -> Result<()>) -> Result<()> {
        match self {
            Self::Variable(variable) => variable.visit(f)?,
            Self::TypeName(_)
            | Self::Unit
            | Self::Bool
            | Self::I8
            | Self::I16
            | Self::I32
            | Self::I64
            | Self::I128
            | Self::U8
            | Self::U16
            | Self::U32
            | Self::U64
            | Self::U128
            | Self::F32
            | Self::F64
            | Self::Char
            | Self::Str
            | Self::Bytes => (),

            Self::Option(format)
            | Self::Seq(format)
            | Self::TupleArray {
                content: format, ..
            } => {
                format.visit(f)?;
            }

            Self::Map { key, value } => {
                key.visit(f)?;
                value.visit(f)?;
            }

            Self::Tuple(formats) => {
                for format in formats {
                    format.visit(f)?;
                }
            }
        }
        f(self)
    }

    fn visit_mut(&mut self, f: &mut dyn FnMut(&mut Format) -> Result<()>) -> Result<()> {
        match self {
            Self::Variable(variable) => {
                variable.visit_mut(f)?;
                // At this point, `variable` is known and points to variable-free content.
                // Remove the variable.
                *self = std::mem::take(variable)
                    .into_inner()
                    .expect("variable is known");
            }
            Self::TypeName(_)
            | Self::Unit
            | Self::Bool
            | Self::I8
            | Self::I16
            | Self::I32
            | Self::I64
            | Self::I128
            | Self::U8
            | Self::U16
            | Self::U32
            | Self::U64
            | Self::U128
            | Self::F32
            | Self::F64
            | Self::Char
            | Self::Str
            | Self::Bytes => (),

            Self::Option(format)
            | Self::Seq(format)
            | Self::TupleArray {
                content: format, ..
            } => {
                format.visit_mut(f)?;
            }

            Self::Map { key, value } => {
                key.visit_mut(f)?;
                value.visit_mut(f)?;
            }

            Self::Tuple(formats) => {
                for format in formats {
                    format.visit_mut(f)?;
                }
            }
        }
        f(self)
    }

    /// Unify the newly "traced" value `format` into the current format.
    /// Note that there should be no `TupleArray`s at this point.
    fn unify(&mut self, format: Format) -> Result<()> {
        match (self, format) {
            (format1, Self::Variable(variable2)) => {
                if let Some(format2) = variable2.borrow_mut().deref_mut() {
                    format1.unify(std::mem::take(format2))?;
                }
                *variable2.borrow_mut() = Some(format1.clone());
            }
            (Self::Variable(variable1), format2) => {
                let inner_variable = match variable1.borrow_mut().deref_mut() {
                    value1 @ None => {
                        *value1 = Some(format2);
                        None
                    }
                    Some(format1) => {
                        format1.unify(format2)?;
                        match format1 {
                            Self::Variable(variable) => Some(variable.clone()),
                            _ => None,
                        }
                    }
                };
                // Reduce multiple indirections to a single one.
                if let Some(variable) = inner_variable {
                    *variable1 = variable;
                }
            }

            (Self::Unit, Self::Unit)
            | (Self::Bool, Self::Bool)
            | (Self::I8, Self::I8)
            | (Self::I16, Self::I16)
            | (Self::I32, Self::I32)
            | (Self::I64, Self::I64)
            | (Self::I128, Self::I128)
            | (Self::U8, Self::U8)
            | (Self::U16, Self::U16)
            | (Self::U32, Self::U32)
            | (Self::U64, Self::U64)
            | (Self::U128, Self::U128)
            | (Self::F32, Self::F32)
            | (Self::F64, Self::F64)
            | (Self::Char, Self::Char)
            | (Self::Str, Self::Str)
            | (Self::Bytes, Self::Bytes) => (),

            (Self::TypeName(name1), Self::TypeName(name2)) if *name1 == name2 => (),

            (Self::Option(format1), Self::Option(format2))
            | (Self::Seq(format1), Self::Seq(format2)) => {
                format1.as_mut().unify(*format2)?;
            }

            (Self::Tuple(formats1), Self::Tuple(formats2)) if formats1.len() == formats2.len() => {
                for (format1, format2) in formats1.iter_mut().zip(formats2.into_iter()) {
                    format1.unify(format2)?;
                }
            }

            (
                Self::Map {
                    key: key1,
                    value: value1,
                },
                Self::Map {
                    key: key2,
                    value: value2,
                },
            ) => {
                key1.as_mut().unify(*key2)?;
                value1.as_mut().unify(*value2)?;
            }

            (format1, format2) => {
                return Err(unification_error(format1, format2));
            }
        }
        Ok(())
    }

    fn is_unknown(&self) -> bool {
        if let Self::Variable(v) = self {
            return v.is_unknown();
        }
        false
    }
}

/// Helper trait to update formats in maps.
pub(crate) trait ContainerFormatEntry {
    fn unify(self, format: ContainerFormat) -> Result<()>;
}

impl<K> ContainerFormatEntry for Entry<'_, K, ContainerFormat>
where
    K: std::cmp::Ord,
{
    fn unify(self, format: ContainerFormat) -> Result<()> {
        match self {
            Entry::Vacant(e) => {
                e.insert(format);
                Ok(())
            }
            Entry::Occupied(e) => e.into_mut().unify(format),
        }
    }
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

// For better rendering in human readable formats, we wish to serialize
// `Named { key: x, value: y }` as a map `{ x: y }`.
impl<T> Serialize for Named<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        if serializer.is_human_readable() {
            let mut map = serializer.serialize_map(Some(1))?;
            map.serialize_entry(&self.name, &self.value)?;
            map.end()
        } else {
            let mut inner = serializer.serialize_struct("Named", 2)?;
            inner.serialize_field("name", &self.name)?;
            inner.serialize_field("value", &self.value)?;
            inner.end()
        }
    }
}

struct NamedVisitor<T> {
    marker: std::marker::PhantomData<T>,
}

impl<T> NamedVisitor<T> {
    fn new() -> Self {
        Self {
            marker: std::marker::PhantomData,
        }
    }
}

impl<'de, T> de::Visitor<'de> for NamedVisitor<T>
where
    T: Deserialize<'de>,
{
    type Value = Named<T>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a single entry map")
    }

    fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
    where
        M: de::MapAccess<'de>,
    {
        let named_value = match access.next_entry::<String, T>()? {
            Some((name, value)) => Named { name, value },
            _ => {
                return Err(de::Error::custom("Missing entry"));
            }
        };
        if access.next_entry::<String, T>()?.is_some() {
            return Err(de::Error::custom("Too many entries"));
        }
        Ok(named_value)
    }
}

/// For deserialization of non-human readable `Named` values, we keep it simple and use derive macros.
#[derive(Deserialize)]
#[serde(rename = "Named")]
struct NamedInternal<T> {
    name: String,
    value: T,
}

impl<'de, T> Deserialize<'de> for Named<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Named<T>, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            deserializer.deserialize_map(NamedVisitor::new())
        } else {
            let NamedInternal { name, value } = NamedInternal::deserialize(deserializer)?;
            Ok(Self { name, value })
        }
    }
}
