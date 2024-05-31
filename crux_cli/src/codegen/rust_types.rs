/// Identifier used in Rust structs, enums, and fields. It includes the `original` name and the `renamed` value after the transformation based on `serde` attributes.
#[derive(Debug, Clone, PartialEq)]
pub struct Id {
    // the identifier from the rustdoc json
    pub id: rustdoc_types::Id,
    /// The original identifier name
    pub original: String,
    /// The renamed identifier, based on serde attributes.
    /// If there is no re-naming going on, this will be identical to
    /// `original`.
    pub renamed: String,
}

impl Id {
    pub fn new(id: rustdoc_types::Id) -> Self {
        Self {
            id,
            original: String::new(),
            renamed: String::new(),
        }
    }
}

impl From<rustdoc_types::Id> for Id {
    fn from(id: rustdoc_types::Id) -> Self {
        Self::new(id)
    }
}

// Rust struct.
#[derive(Debug, Clone, PartialEq)]
pub struct RustStruct {
    /// The identifier for the struct.
    pub id: Id,
    /// The generic parameters that come after the struct name.
    pub generic_types: Vec<String>,
    /// The fields of the struct.
    pub fields: Vec<RustField>,
    /// Comments that were in the struct source.
    /// We copy comments over to the typeshared files,
    /// so we need to collect them here.
    pub comments: Vec<String>,
}

impl RustStruct {
    pub fn new(id: Id) -> Self {
        Self {
            id,
            generic_types: Vec::new(),
            fields: Vec::new(),
            comments: Vec::new(),
        }
    }
}

/// Rust type alias.
/// ```
/// pub struct MasterPassword(String);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct RustTypeAlias {
    /// The identifier for the alias.
    pub id: Id,
    /// The generic parameters that come after the type alias name.
    pub generic_types: Vec<String>,
    /// The type identifier that this type alias is aliasing
    pub r#type: RustType,
    /// Comments that were in the type alias source.
    pub comments: Vec<String>,
}

/// Rust field definition.
#[derive(Debug, Clone, PartialEq)]
pub struct RustField {
    /// Identifier for the field.
    pub id: Id,
    /// Type of the field.
    pub ty: RustType,
    /// Comments that were in the original source.
    pub comments: Vec<String>,
    /// This will be true if the field has a `serde(default)` decorator.
    /// Even if the field's type is not optional, we need to make it optional
    /// for the languages we generate code for.
    pub has_default: bool,
}

/// A Rust type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RustType {
    /// A type with generic parameters. Consists of a type ID + parameters that come
    /// after in angled brackets. Examples include:
    /// - `SomeStruct<String>`
    /// - `SomeEnum<u32>`
    /// - `SomeTypeAlias<(), &str>`
    /// However, there are some generic types that are considered to be _special_. These
    /// include `Vec<T>` `HashMap<K, V>`, and `Option<T>`, which are part of `SpecialRustType` instead
    /// of `RustType::Generic`.
    Generic {
        #[allow(missing_docs)]
        id: String,
        #[allow(missing_docs)]
        parameters: Vec<RustType>,
    },
    /// A type that requires a special transformation to its respective language. This includes
    /// many core types, like string types, basic container types, numbers, and other primitives.
    Special(SpecialRustType),
    /// A type with no generic parameters that is not considered a **special** type. This includes
    /// all user-generated types and some types from the standard library or third-party crates.
    Simple {
        #[allow(missing_docs)]
        id: String,
    },
}

/// A special rust type that needs a manual type conversion
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpecialRustType {
    /// Represents `Vec<T>` from the standard library
    Vec(Box<RustType>),
    /// Represents `[T; N]` from the standard library
    Array(Box<RustType>, usize),
    /// Represents `&[T]` from the standard library
    Slice(Box<RustType>),
    /// Represents `HashMap<K, V>` from the standard library
    HashMap(Box<RustType>, Box<RustType>),
    /// Represents `Option<T>` from the standard library
    Option(Box<RustType>),
    /// Represents `()`
    Unit,
    /// Represents `String` from the standard library
    String,
    /// Represents `char`
    Char,
    /// Represents `i8`
    I8,
    /// Represents `i16`
    I16,
    /// Represents `i32`
    I32,
    /// Represents `i64`
    I64,
    /// Represents `u8`
    U8,
    /// Represents `u16`
    U16,
    /// Represents `u32`
    U32,
    /// Represents `u64`
    U64,
    /// Represents `isize`
    ISize,
    /// Represents `usize`
    USize,
    /// Represents `bool`
    Bool,
    /// Represents `f32`
    F32,
    /// Represents `f64`
    F64,
    /// Represents `I54` from `typeshare::I54`
    I54,
    /// Represents `U53` from `typeshare::U53`
    U53,
}

impl<'a> TryFrom<&'a str> for SpecialRustType {
    type Error = &'a str;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        match value {
            "String" => Ok(SpecialRustType::String),
            "char" => Ok(SpecialRustType::Char),
            "i8" => Ok(SpecialRustType::I8),
            "i16" => Ok(SpecialRustType::I16),
            "i32" => Ok(SpecialRustType::I32),
            "i64" => Ok(SpecialRustType::I64),
            "u8" => Ok(SpecialRustType::U8),
            "u16" => Ok(SpecialRustType::U16),
            "u32" => Ok(SpecialRustType::U32),
            "u64" => Ok(SpecialRustType::U64),
            "isize" => Ok(SpecialRustType::ISize),
            "usize" => Ok(SpecialRustType::USize),
            "bool" => Ok(SpecialRustType::Bool),
            "f32" => Ok(SpecialRustType::F32),
            "f64" => Ok(SpecialRustType::F64),
            "I54" => Ok(SpecialRustType::I54),
            "U53" => Ok(SpecialRustType::U53),
            _ => Err(value),
        }
    }
}

/// Parsed information about a Rust enum definition
#[derive(Debug, Clone, PartialEq)]
pub enum RustEnum {
    /// A unit enum
    ///
    /// An example of such an enum:
    ///
    /// ```
    /// enum UnitEnum {
    ///     Variant,
    ///     AnotherVariant,
    ///     Yay,
    /// }
    /// ```
    Unit(RustEnumShared),
    /// An algebraic enum
    ///
    /// An example of such an enum:
    ///
    /// ```
    /// struct AssociatedData { /* ... */ }
    ///
    /// enum AlgebraicEnum {
    ///     UnitVariant,
    ///     TupleVariant(AssociatedData),
    ///     AnonymousStruct {
    ///         field: String,
    ///         another_field: bool,
    ///     },
    /// }
    /// ```
    Algebraic {
        /// The parsed value of the `#[serde(tag = "...")]` attribute
        tag_key: String,
        /// The parsed value of the `#[serde(content = "...")]` attribute
        content_key: String,
        /// Shared context for this enum.
        shared: RustEnumShared,
    },
}

impl RustEnum {
    /// Get a reference to the inner shared content
    pub fn shared(&self) -> &RustEnumShared {
        match self {
            Self::Unit(shared) | Self::Algebraic { shared, .. } => shared,
        }
    }
}

/// Enum information shared among different enum types
#[derive(Debug, Clone, PartialEq)]
pub struct RustEnumShared {
    /// The enum's ident
    pub id: Id,
    /// Generic parameters for the enum, e.g. `SomeEnum<T>` would produce `vec!["T"]`
    pub generic_types: Vec<String>,
    /// Comments on the enum definition itself
    pub comments: Vec<String>,
    /// The enum's variants
    pub variants: Vec<RustEnumVariant>,
    /// True if this enum references itself in any field of any variant
    /// Swift needs the special keyword `indirect` for this case
    pub is_recursive: bool,
}

/// Parsed information about a Rust enum variant
#[derive(Debug, Clone, PartialEq)]
pub enum RustEnumVariant {
    /// A unit variant
    Unit(RustEnumVariantShared),
    /// A tuple variant
    Tuple {
        /// The type of the single tuple field
        ty: RustType,
        /// Shared context for this enum.
        shared: RustEnumVariantShared,
    },
    /// An anonymous struct variant
    AnonymousStruct {
        /// The fields of the anonymous struct
        fields: Vec<RustField>,
        /// Shared context for this enum.
        shared: RustEnumVariantShared,
    },
}

impl RustEnumVariant {
    /// Get a reference to the inner shared content
    pub fn shared(&self) -> &RustEnumVariantShared {
        match self {
            Self::Unit(shared)
            | Self::Tuple { shared, .. }
            | Self::AnonymousStruct { shared, .. } => shared,
        }
    }
}

/// Variant information shared among different variant types
#[derive(Debug, Clone, PartialEq)]
pub struct RustEnumVariantShared {
    /// The variant's ident
    pub id: Id,
    /// Comments applied to the variant
    pub comments: Vec<String>,
}

/// An enum that encapsulates units of code generation for Typeshare.
/// Analogous to `syn::Item`, even though our variants are more limited.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum RustItem {
    /// A `struct` definition
    Struct(RustStruct),
    /// An `enum` definition
    Enum(RustEnum),
    /// A `type` definition or newtype struct.
    Alias(RustTypeAlias),
}
