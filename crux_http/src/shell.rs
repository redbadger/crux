//! The shell handler `crux_http` ships.
//!
//! The rules for turning a platform's HTTP client into an [`HttpResult`] belong
//! to this crate — which `URLError` is a timeout, that a 404 is an answer and
//! not a failure — so this crate writes them, once per language, and an app
//! asks for them where it configures type generation:
//!
//! ```rust,ignore
//! // shared/src/bin/codegen.rs
//! let typegen = TypeRegistry::new()
//!     .register_app::<MyApp>()?
//!     .shell_handler(&crux_http::HTTP)?
//!     .build()?;
//! ```
//!
//! The shell then holds an instance of the emitted type and delegates to it,
//! one line per operation. Nothing is emitted for an app that does not ask.
//!
//! [`HttpResult`]: crate::protocol::HttpResult

use crux_core::{
    capability::Operation,
    type_generation::facet::{ShellHandler, ShellSource, TypeGenError, TypeRegistry},
};

use crate::protocol::HttpRequest;

/// What `crux_http` ships for the shell.
///
/// The Swift, Kotlin, TypeScript and C# implementations of the crate's
/// protocol, for an app to register with
/// [`TypeRegistry::shell_handler`](crux_core::type_generation::facet::TypeRegistry::shell_handler).
///
/// Each declares an `HttpHandler` protocol (`IHttpHandler` in C#) with one
/// `request` method, and an implementation of it over the platform's own
/// client: `URLSession`, `HttpURLConnection`, `fetch` and `HttpClient`. The
/// client is the shell's choice — conform your own type to the protocol to make
/// a different one — and the protocol rules are the same wherever it comes
/// from.
pub static HTTP: ShellHandler = ShellHandler::new("Http")
    .types(register_types)
    .swift(ShellSource::stdlib(include_str!(
        "../shell/swift/Http.swift"
    )))
    .kotlin(ShellSource::stdlib(include_str!("../shell/kotlin/Http.kt")))
    .typescript(ShellSource::stdlib(include_str!(
        "../shell/typescript/http.ts"
    )))
    .csharp(ShellSource::stdlib(include_str!("../shell/csharp/Http.cs")));

/// Registers the types [`HTTP`]'s sources name, for an app that does not use
/// every one of them itself.
///
/// A shipped source implements the whole capability, so every type it mentions
/// has to be generated — including for operations this app never sends.
/// `crux_http` has one operation, so an app that uses the capability at all has
/// already registered it and this changes nothing; it is here so that every
/// capability is registered the same way.
fn register_types(registry: &mut TypeRegistry) -> Result<&mut TypeRegistry, TypeGenError> {
    HttpRequest::register_types_facet(registry)
}
