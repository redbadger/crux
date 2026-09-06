//! Marker traits declaring what the shell does with an operation.
//!
//! An operation's [`RequestKind`] says how many times a request carrying it
//! expects to be resolved. Declaring it on the operation type — as
//! [`Operation::KIND`] plus one of the markers in this module — lets the
//! `Command` constructors check the kind at compile time, and lets type
//! generation tell the shell what to do with the request.
//!
//! Import the module rather than the items: [`Request`] and [`Stream`] would
//! otherwise collide with [`crux_core::Request`](crate::Request) and
//! `futures::Stream`.
//!
//! ```
//! use crux_core::{RequestKind, capability::Operation, operation};
//! # use facet::Facet;
//! # use serde::{Deserialize, Serialize};
//!
//! /// Told to the shell, never answered.
//! #[derive(Facet, Debug, Clone, Serialize, Deserialize)]
//! pub struct Publish(pub Vec<u8>);
//!
//! impl Operation for Publish {
//!     type Output = ();
//!     const KIND: Option<RequestKind> = Some(RequestKind::Notify);
//! }
//!
//! impl operation::Notify for Publish {}
//!
//! /// Answered exactly once.
//! #[derive(Facet, Debug, Clone, Serialize, Deserialize)]
//! pub struct Get {
//!     pub key: String,
//! }
//!
//! impl Operation for Get {
//!     type Output = Vec<u8>;
//!     const KIND: Option<RequestKind> = Some(RequestKind::Request);
//! }
//!
//! impl operation::Request for Get {}
//!
//! /// Answered a sequence of times.
//! #[derive(Facet, Debug, Clone, Serialize, Deserialize)]
//! pub struct Subscribe;
//!
//! impl Operation for Subscribe {
//!     type Output = Vec<u8>;
//!     const KIND: Option<RequestKind> = Some(RequestKind::Stream);
//! }
//!
//! impl operation::Stream for Subscribe {}
//! ```
//!
//! Declaring a kind is optional. An operation that leaves `KIND` at its
//! default of `None` behaves exactly as it always has: any of the three
//! constructors will take it.

pub use crate::{RequestKind, capability::Operation};

/// An operation the shell is told about and never answers.
///
/// Its requests are built with
/// [`notify_shell`](crate::command::CommandContext::notify_shell), so there is
/// nothing to resolve and the output is `()`.
pub trait Notify: Operation<Output = ()> {}

/// An operation the shell answers exactly once, with an
/// [`Operation::Output`].
///
/// Its requests are built with
/// [`request_from_shell`](crate::command::CommandContext::request_from_shell).
pub trait Request: Operation {}

/// An operation the shell answers a sequence of times, each time with an
/// [`Operation::Output`].
///
/// Its requests are built with
/// [`stream_from_shell`](crate::command::CommandContext::stream_from_shell).
pub trait Stream: Operation {}
