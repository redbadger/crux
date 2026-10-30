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
//!     .shell_extension(&crux_kv::shell::swift::IN_MEMORY)?
//!     .shell_extension(&crux_kv::shell::kotlin::IN_MEMORY)?
//!     .shell_extension(&crux_kv::shell::typescript::IN_MEMORY)?
//!     .shell_extension(&crux_kv::shell::csharp::IN_MEMORY)?
//!     .build()?;
//! ```
//!
//! The shell then holds an instance of the emitted type and delegates to it,
//! one line per operation. Nothing is emitted for an app that does not ask.
//!
//! The Swift, Kotlin, TypeScript and C# implementations of the crate's
//! protocol each declare a `KeyValueHandler` protocol (`IKeyValueHandler` in
//! C#) with one method per operation, and one implementation of it,
//! `InMemoryKeyValueHandler` (`createInMemoryKeyValueHandler` in TypeScript),
//! which keeps values for the life of the handler. The durable store is the
//! shell's choice, because the right store differs on each platform. To keep
//! values after the app stops, a shell conforms its own type to the protocol.
//! The weather and notes examples show durable stores for each platform.
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

    /// `KeyValueHandler`, and `InMemoryKeyValueHandler`.
    pub static IN_MEMORY: ShellExtension = ShellExtension::swift(
        "KeyValue",
        include_dir!("$CARGO_MANIFEST_DIR/shell/swift/in_memory"),
    )
    .platforms(&[Platform::Apple, Platform::Linux])
    .rust_types(super::register_types)
    // The names the shipped sources declare at module scope.
    .foreign_types(&["KeyValueHandler", "InMemoryKeyValueHandler"]);
}

/// The Kotlin handler, for an app to register with
/// [`TypeRegistry::shell_extension`](crux_core::type_generation::facet::TypeRegistry::shell_extension).
pub mod kotlin {
    use crux_core::type_generation::facet::{Platform, ShellExtension};
    use include_dir::include_dir;

    /// `KeyValueHandler`, and `InMemoryKeyValueHandler`.
    pub static IN_MEMORY: ShellExtension = ShellExtension::kotlin(
        "KeyValue",
        include_dir!("$CARGO_MANIFEST_DIR/shell/kotlin/in_memory"),
    )
    .platforms(&[Platform::Jvm, Platform::Android])
    .rust_types(super::register_types)
    // The names the shipped sources declare at module scope.
    .foreign_types(&["KeyValueHandler", "InMemoryKeyValueHandler"]);
}

/// The TypeScript handler, for an app to register with
/// [`TypeRegistry::shell_extension`](crux_core::type_generation::facet::TypeRegistry::shell_extension).
pub mod typescript {
    use crux_core::type_generation::facet::{Platform, ShellExtension};
    use include_dir::include_dir;

    /// `KeyValueHandler`, and `createInMemoryKeyValueHandler`.
    pub static IN_MEMORY: ShellExtension = ShellExtension::typescript(
        "KeyValue",
        include_dir!("$CARGO_MANIFEST_DIR/shell/typescript/in_memory"),
    )
    .platforms(&[Platform::Web])
    .rust_types(super::register_types)
    // The names the shipped sources declare at module scope.
    .foreign_types(&["KeyValueHandler", "createInMemoryKeyValueHandler"]);
}

/// The C# handler, for an app to register with
/// [`TypeRegistry::shell_extension`](crux_core::type_generation::facet::TypeRegistry::shell_extension).
pub mod csharp {
    use crux_core::type_generation::facet::{Platform, ShellExtension};
    use include_dir::include_dir;

    /// `IKeyValueHandler`, and `InMemoryKeyValueHandler`.
    pub static IN_MEMORY: ShellExtension = ShellExtension::csharp(
        "KeyValue",
        include_dir!("$CARGO_MANIFEST_DIR/shell/csharp/in_memory"),
    )
    .platforms(&[Platform::DotNet])
    .rust_types(super::register_types)
    // The names the shipped sources declare at module scope.
    .foreign_types(&["IKeyValueHandler", "InMemoryKeyValueHandler"]);
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
