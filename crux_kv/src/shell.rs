//! The shell handler `crux_kv` ships, as shell extensions.
//!
//! The rules for answering a store operation belong to this crate — that a
//! missing key is a [`Value::None`] and not an error, that `Set` and `Delete`
//! answer with what they replaced — so this crate writes them, once per
//! language, and an app asks for them where it configures type generation:
//!
//! ```rust,ignore
//! // shared/src/bin/codegen.rs
//! let typegen = TypeRegistry::new()
//!     .register_app::<MyApp>()?
//!     .shell_extension(&crux_kv::shell::swift::USER_DEFAULTS)?
//!     .shell_extension(&crux_kv::shell::kotlin::FILE)?
//!     .shell_extension(&crux_kv::shell::typescript::LOCAL_STORAGE)?
//!     .shell_extension(&crux_kv::shell::csharp::FILE)?
//!     .build()?;
//! ```
//!
//! The shell then holds an instance of the emitted type and delegates to it,
//! one line per operation. Nothing is emitted for an app that does not ask.
//!
//! The Swift, Kotlin, TypeScript and C# implementations of the crate's
//! protocol each declare a `KeyValueHandler` protocol (`IKeyValueHandler` in
//! C#) with one method per operation, and an implementation of it over a store
//! every platform already has: `UserDefaults` on Apple platforms, a file per
//! key on the JVM and in C#, and `localStorage` in the browser. Where the store
//! lives is the shell's choice — each implementation takes it as
//! configuration, and a shell that wants a different one conforms its own type
//! to the protocol.
//!
//! [`Value::None`]: crate::protocol::Value::None

use crux_core::{
    capability::Operation,
    type_generation::facet::{TypeGenError, TypeRegistry},
};

use crate::operation::{Delete, Exists, Get, ListKeys, Set};

/// The Swift handler, for an app to register with
/// [`TypeRegistry::shell_extension`](crux_core::type_generation::facet::TypeRegistry::shell_extension).
pub mod swift {
    use crux_core::type_generation::facet::{Platform, ShellExtension};
    use include_dir::include_dir;

    /// `KeyValueHandler`, and `UserDefaultsKeyValueHandler` over `UserDefaults`.
    pub static USER_DEFAULTS: ShellExtension = ShellExtension::swift(
        "KeyValue",
        include_dir!("$CARGO_MANIFEST_DIR/shell/swift/user_defaults"),
    )
    .platforms(&[Platform::Apple, Platform::Linux])
    .rust_types(super::register_types)
    // The names the shipped sources declare at module scope.
    .foreign_types(&["KeyValueHandler", "UserDefaultsKeyValueHandler"]);
}

/// The Kotlin handler, for an app to register with
/// [`TypeRegistry::shell_extension`](crux_core::type_generation::facet::TypeRegistry::shell_extension).
pub mod kotlin {
    use crux_core::type_generation::facet::{Platform, ShellExtension};
    use include_dir::include_dir;

    /// `KeyValueHandler`, and `FileKeyValueHandler` over a file per key.
    pub static FILE: ShellExtension = ShellExtension::kotlin(
        "KeyValue",
        include_dir!("$CARGO_MANIFEST_DIR/shell/kotlin/file"),
    )
    .platforms(&[Platform::Jvm, Platform::Android])
    .rust_types(super::register_types)
    // The names the shipped sources declare at module scope.
    .foreign_types(&["KeyValueHandler", "FileKeyValueHandler"]);
}

/// The TypeScript handler, for an app to register with
/// [`TypeRegistry::shell_extension`](crux_core::type_generation::facet::TypeRegistry::shell_extension).
pub mod typescript {
    use crux_core::type_generation::facet::{Platform, ShellExtension};
    use include_dir::include_dir;

    /// `KeyValueHandler`, and `createLocalStorageKeyValueHandler` over
    /// `localStorage`.
    pub static LOCAL_STORAGE: ShellExtension = ShellExtension::typescript(
        "KeyValue",
        include_dir!("$CARGO_MANIFEST_DIR/shell/typescript/local_storage"),
    )
    .platforms(&[Platform::Web])
    .rust_types(super::register_types)
    // The names the shipped sources declare at module scope.
    .foreign_types(&[
        "KeyValueHandler",
        "createLocalStorageKeyValueHandler",
        "localStorageKeyValueHandler",
    ]);
}

/// The C# handler, for an app to register with
/// [`TypeRegistry::shell_extension`](crux_core::type_generation::facet::TypeRegistry::shell_extension).
pub mod csharp {
    use crux_core::type_generation::facet::{Platform, ShellExtension};
    use include_dir::include_dir;

    /// `IKeyValueHandler`, and `FileKeyValueHandler` over a file per key.
    pub static FILE: ShellExtension = ShellExtension::csharp(
        "KeyValue",
        include_dir!("$CARGO_MANIFEST_DIR/shell/csharp/file"),
    )
    .platforms(&[Platform::DotNet])
    .rust_types(super::register_types)
    // The names the shipped sources declare at module scope.
    .foreign_types(&["IKeyValueHandler", "FileKeyValueHandler"]);
}

/// Registers the types the shipped sources name, for an app that does not use
/// every one of them itself.
///
/// A shipped source implements the whole capability, so every type it mentions
/// has to be generated — including for the operations this app never sends. An
/// app that only reads and writes still has `Delete`, `Exists` and `ListKeys`
/// in its generated module, because the shipped `KeyValueHandler` implements
/// them.
///
/// `Delete` is the one to watch: it is an obvious name for an app's own
/// operation too, and two types generating as `Delete` is one type in the
/// generated module.
fn register_types(registry: &mut TypeRegistry) -> Result<&mut TypeRegistry, TypeGenError> {
    Get::register_types_facet(registry)?;
    Set::register_types_facet(registry)?;
    Delete::register_types_facet(registry)?;
    Exists::register_types_facet(registry)?;
    ListKeys::register_types_facet(registry)
}
