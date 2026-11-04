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
//!
//! # Implementations
//!
//! The crate also ships an implementation of the protocol in each language,
//! over the platform's own client: `URLSessionHttpHandler` over `URLSession`,
//! `UrlConnectionHttpHandler` over `HttpURLConnection`,
//! `createFetchHttpHandler` over `fetch` and `HttpClientHttpHandler` over
//! `HttpClient`. An implementation is a shell extension, and an app gets one
//! only if it registers it where it configures type generation:
//!
//! ```rust,ignore
//! // shared/src/bin/codegen.rs
//! let typegen = TypeRegistry::new()
//!     .register_app::<MyApp>()?
//!     .shell_extension(&crux_http::shell::swift::URL_SESSION)?
//!     .shell_extension(&crux_http::shell::kotlin::URL_CONNECTION)?
//!     .shell_extension(&crux_http::shell::typescript::FETCH)?
//!     .shell_extension(&crux_http::shell::csharp::HTTP_CLIENT)?
//!     .build()?;
//! ```
//!
//! The shell then holds an instance of the emitted type and delegates the
//! operation to it in one line. To use a different client, implement the
//! protocol over it, and register no extension.
//!
//! The implementations need the `facet_typegen` feature of `crux_http`.

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
    /// module whether or not the app sends the operations. Type generation
    /// rejects an app type that generates under one of these names.
    ///
    /// # Errors
    /// Returns a [`TypeGenError`] if a type cannot be registered.
    pub fn register_types(registry: &mut TypeRegistry) -> Result<&mut TypeRegistry, TypeGenError> {
        HttpRequest::register_types_facet(registry)
    }
}

/// The Swift implementation, for an app to register with
/// [`TypeRegistry::shell_extension`](crux_core::type_generation::facet::TypeRegistry::shell_extension).
#[cfg(feature = "facet_typegen")]
pub mod swift {
    use crux_core::type_generation::facet::{Platform, ShellExtension};
    use include_dir::include_dir;

    /// `URLSessionHttpHandler`, over `URLSession`.
    pub static URL_SESSION: ShellExtension = ShellExtension::swift(
        "Http",
        include_dir!("$CARGO_MANIFEST_DIR/shell/swift/url_session"),
    )
    .platforms(&[Platform::Apple, Platform::Linux])
    .protocol(&super::PROTOCOL)
    // The names the shipped sources declare at module scope.
    .foreign_types(&["URLSessionHttpHandler"]);
}

/// The Kotlin implementation, for an app to register with
/// [`TypeRegistry::shell_extension`](crux_core::type_generation::facet::TypeRegistry::shell_extension).
#[cfg(feature = "facet_typegen")]
pub mod kotlin {
    use crux_core::type_generation::facet::{Platform, ShellExtension};
    use include_dir::include_dir;

    /// `UrlConnectionHttpHandler` over `HttpURLConnection`, with `CookieJar`
    /// and `InMemoryCookieJar`. On Android, also `WebkitCookieJar`, which
    /// keeps cookies in `android.webkit.CookieManager`.
    pub static URL_CONNECTION: ShellExtension = ShellExtension::kotlin(
        "Http",
        include_dir!("$CARGO_MANIFEST_DIR/shell/kotlin/url_connection"),
    )
    .platforms(&[Platform::Jvm, Platform::Android])
    .protocol(&super::PROTOCOL)
    // The names the shipped sources declare at module scope.
    .foreign_types(&[
        "CookieJar",
        "InMemoryCookieJar",
        "UrlConnectionHttpHandler",
        "WebkitCookieJar",
    ]);
}

/// The TypeScript implementation, for an app to register with
/// [`TypeRegistry::shell_extension`](crux_core::type_generation::facet::TypeRegistry::shell_extension).
#[cfg(feature = "facet_typegen")]
pub mod typescript {
    use crux_core::type_generation::facet::{Platform, ShellExtension};
    use include_dir::include_dir;

    /// `createFetchHttpHandler`, over `fetch`.
    pub static FETCH: ShellExtension = ShellExtension::typescript(
        "Http",
        include_dir!("$CARGO_MANIFEST_DIR/shell/typescript/fetch"),
    )
    .platforms(&[Platform::Web])
    .protocol(&super::PROTOCOL)
    // The names the shipped sources declare at module scope.
    .foreign_types(&["createFetchHttpHandler"]);
}

/// The C# implementation, for an app to register with
/// [`TypeRegistry::shell_extension`](crux_core::type_generation::facet::TypeRegistry::shell_extension).
#[cfg(feature = "facet_typegen")]
pub mod csharp {
    use crux_core::type_generation::facet::{Platform, ShellExtension};
    use include_dir::include_dir;

    /// `HttpClientHttpHandler`, over `HttpClient`.
    pub static HTTP_CLIENT: ShellExtension = ShellExtension::csharp(
        "Http",
        include_dir!("$CARGO_MANIFEST_DIR/shell/csharp/http_client"),
    )
    .platforms(&[Platform::DotNet])
    .protocol(&super::PROTOCOL)
    // The names the shipped sources declare at module scope.
    .foreign_types(&["HttpClientHttpHandler"]);
}
