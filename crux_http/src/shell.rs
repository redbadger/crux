//! The shell handler `crux_http` ships, as shell extensions.
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
//!     .shell_extension(&crux_http::shell::swift::URL_SESSION)?
//!     .shell_extension(&crux_http::shell::kotlin::URL_CONNECTION)?
//!     .shell_extension(&crux_http::shell::typescript::FETCH)?
//!     .shell_extension(&crux_http::shell::csharp::HTTP_CLIENT)?
//!     .build()?;
//! ```
//!
//! The shell then holds an instance of the emitted type and delegates to it,
//! one line per operation. Nothing is emitted for an app that does not ask.
//!
//! The Swift, Kotlin, TypeScript and C# implementations of the crate's
//! protocol each declare an `HttpHandler` protocol (`IHttpHandler` in C#) with
//! one `request` method, and an implementation of it over the platform's own
//! client: `URLSession`, `HttpURLConnection`, `fetch` and `HttpClient`. The
//! client is the shell's choice — conform your own type to the protocol to make
//! a different one — and the protocol rules are the same wherever it comes
//! from.
//!
//! [`HttpResult`]: crate::protocol::HttpResult

use crux_core::{
    capability::Operation,
    type_generation::facet::{TypeGenError, TypeRegistry},
};

use crate::protocol::HttpRequest;

/// The Swift handler, for an app to register with
/// [`TypeRegistry::shell_extension`](crux_core::type_generation::facet::TypeRegistry::shell_extension).
pub mod swift {
    use crux_core::type_generation::facet::{Platform, ShellExtension};
    use include_dir::include_dir;

    /// `HttpHandler`, and `URLSessionHttpHandler` over `URLSession`.
    pub static URL_SESSION: ShellExtension = ShellExtension::swift(
        "Http",
        include_dir!("$CARGO_MANIFEST_DIR/shell/swift/url_session"),
    )
    .platforms(&[Platform::Apple, Platform::Linux])
    .rust_types(super::register_types)
    // The names the shipped sources declare at module scope.
    .foreign_types(&["HttpHandler", "URLSessionHttpHandler"]);
}

/// The Kotlin handler, for an app to register with
/// [`TypeRegistry::shell_extension`](crux_core::type_generation::facet::TypeRegistry::shell_extension).
pub mod kotlin {
    use crux_core::type_generation::facet::{Platform, ShellExtension};
    use include_dir::include_dir;

    /// `HttpHandler`, and `UrlConnectionHttpHandler` over `HttpURLConnection`.
    /// On Android, also `WebkitCookieJar`, which keeps cookies in
    /// `android.webkit.CookieManager`.
    pub static URL_CONNECTION: ShellExtension = ShellExtension::kotlin(
        "Http",
        include_dir!("$CARGO_MANIFEST_DIR/shell/kotlin/url_connection"),
    )
    .platforms(&[Platform::Jvm, Platform::Android])
    .rust_types(super::register_types)
    // The names the shipped sources declare at module scope.
    .foreign_types(&[
        "HttpHandler",
        "CookieJar",
        "InMemoryCookieJar",
        "UrlConnectionHttpHandler",
        "WebkitCookieJar",
    ]);
}

/// The TypeScript handler, for an app to register with
/// [`TypeRegistry::shell_extension`](crux_core::type_generation::facet::TypeRegistry::shell_extension).
pub mod typescript {
    use crux_core::type_generation::facet::{Platform, ShellExtension};
    use include_dir::include_dir;

    /// `HttpHandler`, and `createFetchHttpHandler` over `fetch`.
    pub static FETCH: ShellExtension = ShellExtension::typescript(
        "Http",
        include_dir!("$CARGO_MANIFEST_DIR/shell/typescript/fetch"),
    )
    .platforms(&[Platform::Web])
    .rust_types(super::register_types)
    // The names the shipped sources declare at module scope.
    .foreign_types(&["HttpHandler", "createFetchHttpHandler"]);
}

/// The C# handler, for an app to register with
/// [`TypeRegistry::shell_extension`](crux_core::type_generation::facet::TypeRegistry::shell_extension).
pub mod csharp {
    use crux_core::type_generation::facet::{Platform, ShellExtension};
    use include_dir::include_dir;

    /// `IHttpHandler`, and `HttpClientHttpHandler` over `HttpClient`.
    pub static HTTP_CLIENT: ShellExtension = ShellExtension::csharp(
        "Http",
        include_dir!("$CARGO_MANIFEST_DIR/shell/csharp/http_client"),
    )
    .platforms(&[Platform::DotNet])
    .rust_types(super::register_types)
    // The names the shipped sources declare at module scope.
    .foreign_types(&["IHttpHandler", "HttpClientHttpHandler"]);
}

/// Registers the types the shipped sources name, for an app that does not use
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
