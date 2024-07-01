//! Contains all token handling logic.
#[cfg(doc)]
use crate::public_item::PublicItem;

/// A token in a rendered [`PublicItem`], used to apply syntax coloring in downstream applications.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Token {
    /// A symbol, like `=` or `::<`
    Symbol(String),
    /// A qualifier, like `pub` or `const`
    Qualifier(String),
    /// The kind of an item, like `function` or `trait`
    Kind(String),
    /// Whitespace, a single space
    Whitespace,
    /// An identifier, like variable names or parts of the path of an item
    Identifier(String),
    /// An annotation, used e.g. for Rust attributes.
    Annotation(String),
    /// The identifier self, the text can be `self` or `Self`
    Self_(String),
    /// The identifier for a function, like `fn_arg` in `comprehensive_api::functions::fn_arg`
    Function(String),
    /// A lifetime including the apostrophe `'`, like `'a`
    Lifetime(String),
    /// A keyword, like `impl`, `where`, or `dyn`
    Keyword(String),
    /// A generic parameter, like `T`
    Generic(String),
    /// A primitive type, like `usize`
    Primitive(String),
    /// A non-primitive type, like the name of a struct or a trait
    Type(String),
}

impl Token {
    /// A symbol, like `=` or `::<`
    pub(crate) fn symbol(text: impl Into<String>) -> Self {
        Self::Symbol(text.into())
    }
    /// A qualifier, like `pub` or `const`
    pub(crate) fn qualifier(text: impl Into<String>) -> Self {
        Self::Qualifier(text.into())
    }
    /// The kind of an item, like `function` or `trait`
    pub(crate) fn kind(text: impl Into<String>) -> Self {
        Self::Kind(text.into())
    }
    /// An identifier, like variable names or parts of the path of an item
    pub(crate) fn identifier(text: impl Into<String>) -> Self {
        Self::Identifier(text.into())
    }
    /// The identifier self, the text can be `self` or `Self`
    pub(crate) fn self_(text: impl Into<String>) -> Self {
        Self::Self_(text.into())
    }
    /// The identifier for a function, like `fn_arg` in `comprehensive_api::functions::fn_arg`
    pub(crate) fn function(text: impl Into<String>) -> Self {
        Self::Function(text.into())
    }
    /// A lifetime including the apostrophe `'`, like `'a`
    pub(crate) fn lifetime(text: impl Into<String>) -> Self {
        Self::Lifetime(text.into())
    }
    /// A keyword, like `impl`
    pub(crate) fn keyword(text: impl Into<String>) -> Self {
        Self::Keyword(text.into())
    }
    /// A generic, like `T`
    pub(crate) fn generic(text: impl Into<String>) -> Self {
        Self::Generic(text.into())
    }
    /// A primitive type, like `usize`
    pub(crate) fn primitive(text: impl Into<String>) -> Self {
        Self::Primitive(text.into())
    }
    /// A type, like `Iterator`
    pub(crate) fn type_(text: impl Into<String>) -> Self {
        Self::Type(text.into())
    }
    /// Give the length of the inner text of this token
    #[allow(clippy::len_without_is_empty)]
    #[must_use]
    pub fn len(&self) -> usize {
        self.text().len()
    }
    /// Get the inner text of this token
    #[must_use]
    pub fn text(&self) -> &str {
        match self {
            Self::Symbol(l)
            | Self::Qualifier(l)
            | Self::Kind(l)
            | Self::Identifier(l)
            | Self::Annotation(l)
            | Self::Self_(l)
            | Self::Function(l)
            | Self::Lifetime(l)
            | Self::Keyword(l)
            | Self::Generic(l)
            | Self::Primitive(l)
            | Self::Type(l) => l,
            Self::Whitespace => " ",
        }
    }
}

pub(crate) fn tokens_to_string(tokens: &[Token]) -> String {
    tokens.iter().map(Token::text).collect()
}
