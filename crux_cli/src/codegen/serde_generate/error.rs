// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

#![allow(dead_code)]

use serde::{de, ser};
use std::fmt;
use thiserror::Error;

use super::format::ContainerFormat;

/// Result type used in this crate.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Error type used in this crate.
#[derive(Clone, Debug, Error, PartialEq)]
pub enum Error {
    #[error("{0}")]
    Custom(String),
    #[error("Not supported: {0}")]
    NotSupported(&'static str),
    #[error("Failed to deserialize {0}")]
    Deserialization(&'static str),
    #[error("In container {0}, recorded value for serialization format {1:?} failed to deserialize into {2}")]
    UnexpectedDeserializationFormat(&'static str, ContainerFormat, &'static str),
    #[error("Incompatible formats detected: {0} {1}")]
    Incompatible(String, String),
    #[error("Incomplete tracing detected")]
    UnknownFormat,
    #[error("Incomplete tracing detected inside container: {0}")]
    UnknownFormatInContainer(String),
    #[error("Missing variants detected for specific enums: {0:?}")]
    MissingVariants(Vec<String>),
}

impl ser::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error::Custom(format!("Failed to serialize value: \"{}\"", msg))
    }
}

impl de::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error::Custom(format!("Failed to deserialize value: \"{}\"", msg))
    }
}

impl Error {
    /// Provides a longer description of the possible cause of an error during tracing.
    pub fn explanation(&self) -> String {
        use Error::*;

        match self {
            Custom(_) => {
                r#"
An error was returned by a Serde trait during (de)serialization tracing. In practice, this happens when
user-provided code 'impl<'de> Deserialize<'de> for Foo { .. }' rejects a candidate value of type `Foo`
provided by serde-reflection.

To fix this, add a call `tracer.trace_value(foo, &mut samples)` so that a correct value `foo` is
recorded *before* `tracer.trace_type` is called.
"#.to_string()
            }
            NotSupported(_) => {
                r#"
An unsupported callback was called during (de)serialization tracing. In practice, this happens when an
unsupported Serde attribute is used. Attributes specific to self-describing formats (JSON, YAML, TOML)
are generally not supported. This includes: `#[serde(flatten)]`, `#[serde(tag = "type")]`,
`#[serde(tag = "t", content = "c")]`, and `#[serde(untagged)]`.

To fix this, avoid unsupported Serde attributes or use custom (de)serialize implementations with different
behaviors depending on the Serde callback `(De)Serializer::is_human_readable()`.
"#.to_string()
            }
            Deserialization(_) => {
                r#"
This internal error should not be surfaced during tracing.
"#.to_string()
            }
            UnexpectedDeserializationFormat(_, _, _) => {
                r#"
A value recorded by `trace_value` fails to deserialize as expected during a `trace_type` call. This can
happen if custom implementations of the Serialize and Deserialize traits do not agree.

Verify the implementations of Serialize and Deserialize for the given format.
"#.to_string()
            }

            Incompatible(_, _) => {
                r#"
Two formats computed for the same entity do not match. This can happen if custom implementations of
the Serialize and Deserialize traits do not agree, e.g. if one uses `Vec<u8>` and the other one uses `&[u8]`
(implying bytes) --- see the crate `serde_bytes` for more context in this particular example.

Verify the implementations of Serialize and Deserialize for the given format.
"#.to_string()
            }
            UnknownFormat => {
                r#"
This internal error is returned should not be surfaced during tracing.
"#.to_string()
            }
            UnknownFormatInContainer(name) => {
                format!(r#"
A final registry was requested but some formats are still unknown within the container {}. This can
happen if `tracer.trace_value` was called on a value `foo` which does not reveal some of the underlying
formats. E.g. if a field `x` of struct `Foo` has type `Option<String>` and `foo` is value of type
`Foo` such that `foo.x == None`, then tracing the value `foo` may result in a format `Option<Unknown>`
for the field `x`. The same applies to empty vectors and empty maps.

To fix this, avoid `trace_value` and prefer `trace_type` when possible, or make sure to trace at
least one value `foo` such that `foo.x` is not empty.
"#,
                name)
            }
            MissingVariants(names) => {
                format!(r#"
A registry was requested with `tracer.registry()` but some variants have not been analyzed yet
inside the given enums {:?}.

To fix this, make sure to call `tracer.trace_type<T>(..)` at least once for each enum type `T` in the
corpus of definitions. You may also use `tracer.registry_unchecked()` for debugging.
"#,
                names)
            }
        }
    }
}
