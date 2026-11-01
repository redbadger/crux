//! The shell protocol of `crux_http`.
//!
//! The protocol is the interface that a shell implements for this capability:
//! `HttpHandler` (`IHttpHandler` in C#), with one method for each operation.
//! Type generation emits it in Swift, Kotlin, TypeScript and C# for every app
//! that uses an operation of `crux_http`. The app does not register it. Each
//! operation names it with `protocol = crate::shell::PROTOCOL` in its
//! `#[operation(..)]` attribute.
//!
//! The protocol follows the `facet_typegen` feature of `crux_core`. The items
//! of this module, and the part of each operation that registers the
//! protocol, exist only when that feature is on. Thus an app that turns on
//! `crux_core/facet_typegen` for type generation gets the protocol, and needs
//! no feature of `crux_http` for it.
//!
//! A response with a 4xx or 5xx status is an answer, not an error. Only a
//! transport failure is an [`HttpError`](crate::HttpError). The protocol
//! states these rules in each language.

crux_core::__crux_core_facet_typegen_items! {
    use crux_core::{
        capability::Operation as _,
        type_generation::facet::{ShellProtocol, TypeGenError, TypeRegistry},
    };

    use crate::protocol::HttpRequest;

    /// The protocol of `crux_http`, in each of the four languages.
    pub static PROTOCOL: ShellProtocol = ShellProtocol::new("Http")
        .rust_types(register_types)
        .swift(include_str!("../shell/swift/Http.swift"), &["HttpHandler"])
        .kotlin(include_str!("../shell/kotlin/Http.kt"), &["HttpHandler"])
        .typescript(
            include_str!("../shell/typescript/http.ts"),
            &["HttpHandler"],
        )
        .csharp(include_str!("../shell/csharp/Http.cs"), &["IHttpHandler"]);

    /// Registers every operation of `crux_http` and its output.
    ///
    /// A shell implements the whole protocol, so every type that the protocol
    /// names must be generated, including the types of the operations that the
    /// app never sends. Thus these names are taken in the app's generated
    /// module whether or not the app sends the operations. Do not give an app
    /// type one of these names.
    ///
    /// # Errors
    /// Returns a [`TypeGenError`] if a type cannot be registered.
    pub fn register_types(registry: &mut TypeRegistry) -> Result<&mut TypeRegistry, TypeGenError> {
        HttpRequest::register_types_facet(registry)
    }
}
