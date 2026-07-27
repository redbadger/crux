//! Helpers for testing apps that use `crux_http`.
//!
//! The two entry points mirror the two things a feature can receive:
//!
//! - [`ResponseBuilder`] builds the `Ok(Response)` of a successful exchange.
//! - [`rejection`] builds the `Err(HttpError::Http { .. })` of a 4xx/5xx rejection, body
//!   included — or [`rejection_from`], when the rejection's headers matter too.
//!
//! There is no third case: a `Response` never carries an error status, so
//! `ResponseBuilder` refuses to build one.

mod rejection;
mod response_builder;

#[cfg(test)]
mod fake_shell;

pub use rejection::{rejection, rejection_from};
pub use response_builder::ResponseBuilder;

#[cfg(test)]
pub(crate) use fake_shell::FakeShell;
