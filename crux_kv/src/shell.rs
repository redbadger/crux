//! The shell protocol of `crux_kv`.
//!
//! The protocol is the interface that a shell implements for this capability:
//! `KeyValueHandler` (`IKeyValueHandler` in C#), with one method for each
//! operation. Type generation emits it in Swift, Kotlin, TypeScript and C# for
//! every app that uses an operation of `crux_kv`. The app does not register it.
//! Each operation names it with `protocol = crate::shell::PROTOCOL` in its
//! `#[operation(..)]` attribute.
//!
//! The protocol follows the `facet_typegen` feature of `crux_core`. The items
//! of this module, and the part of each operation that registers the
//! protocol, exist only when that feature is on. Thus an app that turns on
//! `crux_core/facet_typegen` for type generation gets the protocol, and needs
//! no feature of `crux_kv` for it.
//!
//! A missing key is a [`Value::None`] and not an error, and `SetValue` and
//! `DeleteValue` answer with the value that they replaced or removed. The
//! protocol states these rules in each language.
//!
//! # Implementations
//!
//! The crate also ships one implementation of the protocol in each language:
//! `InMemoryKeyValueHandler` (`createInMemoryKeyValueHandler` in TypeScript),
//! which keeps values only for the life of the handler. Use it in tests, in
//! previews and in a shell with no storage. An implementation is a shell
//! extension, and an app gets one only if it registers it where it configures
//! type generation:
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
//! The right durable store is different on each platform, so the shell
//! chooses it. To keep values after the app stops, implement the protocol
//! over a store that persists. The weather and notes examples show durable
//! stores for each platform.
//!
//! The implementations need the `facet_typegen` feature of `crux_kv`.
//!
//! [`Value::None`]: crate::protocol::Value::None

crux_core::__crux_core_facet_typegen_items! {
    use crux_core::{
        capability::Operation as _,
        type_generation::facet::{ShellProtocol, TypeGenError, TypeRegistry},
    };

    use crate::operation::{DeleteValue, GetValue, KeyExists, ListKeys, SetValue};

    /// The protocol of `crux_kv`, in each of the four languages.
    pub static PROTOCOL: ShellProtocol = ShellProtocol::new("KeyValue")
        .rust_types(register_types)
        .swift(
            include_str!("../shell/swift/KeyValue.swift"),
            &["KeyValueHandler"],
        )
        .kotlin(
            include_str!("../shell/kotlin/KeyValue.kt"),
            &["KeyValueHandler"],
        )
        .typescript(
            include_str!("../shell/typescript/keyValue.ts"),
            &["KeyValueHandler"],
        )
        .csharp(
            include_str!("../shell/csharp/KeyValue.cs"),
            &["IKeyValueHandler"],
        );

    /// Registers every operation of `crux_kv` and its output.
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
        GetValue::register_types_facet(registry)?;
        SetValue::register_types_facet(registry)?;
        DeleteValue::register_types_facet(registry)?;
        KeyExists::register_types_facet(registry)?;
        ListKeys::register_types_facet(registry)
    }
}

/// The Swift implementation, for an app to register with
/// [`TypeRegistry::shell_extension`](crux_core::type_generation::facet::TypeRegistry::shell_extension).
#[cfg(feature = "facet_typegen")]
pub mod swift {
    use crux_core::type_generation::facet::{Platform, ShellExtension};
    use include_dir::include_dir;

    /// `InMemoryKeyValueHandler`, which keeps values in memory.
    pub static IN_MEMORY: ShellExtension = ShellExtension::swift(
        "KeyValue",
        include_dir!("$CARGO_MANIFEST_DIR/shell/swift/in_memory"),
    )
    .platforms(&[Platform::Apple, Platform::Linux])
    .protocol(&super::PROTOCOL)
    // The names the shipped sources declare at module scope.
    .foreign_types(&["InMemoryKeyValueHandler"]);
}

/// The Kotlin implementation, for an app to register with
/// [`TypeRegistry::shell_extension`](crux_core::type_generation::facet::TypeRegistry::shell_extension).
#[cfg(feature = "facet_typegen")]
pub mod kotlin {
    use crux_core::type_generation::facet::{Platform, ShellExtension};
    use include_dir::include_dir;

    /// `InMemoryKeyValueHandler`, which keeps values in memory.
    pub static IN_MEMORY: ShellExtension = ShellExtension::kotlin(
        "KeyValue",
        include_dir!("$CARGO_MANIFEST_DIR/shell/kotlin/in_memory"),
    )
    .platforms(&[Platform::Jvm, Platform::Android])
    .protocol(&super::PROTOCOL)
    // The names the shipped sources declare at module scope.
    .foreign_types(&["InMemoryKeyValueHandler"]);
}

/// The TypeScript implementation, for an app to register with
/// [`TypeRegistry::shell_extension`](crux_core::type_generation::facet::TypeRegistry::shell_extension).
#[cfg(feature = "facet_typegen")]
pub mod typescript {
    use crux_core::type_generation::facet::{Platform, ShellExtension};
    use include_dir::include_dir;

    /// `createInMemoryKeyValueHandler`, which keeps values in memory.
    pub static IN_MEMORY: ShellExtension = ShellExtension::typescript(
        "KeyValue",
        include_dir!("$CARGO_MANIFEST_DIR/shell/typescript/in_memory"),
    )
    .platforms(&[Platform::Web])
    .protocol(&super::PROTOCOL)
    // The names the shipped sources declare at module scope.
    .foreign_types(&["createInMemoryKeyValueHandler"]);
}

/// The C# implementation, for an app to register with
/// [`TypeRegistry::shell_extension`](crux_core::type_generation::facet::TypeRegistry::shell_extension).
#[cfg(feature = "facet_typegen")]
pub mod csharp {
    use crux_core::type_generation::facet::{Platform, ShellExtension};
    use include_dir::include_dir;

    /// `InMemoryKeyValueHandler`, which keeps values in memory.
    pub static IN_MEMORY: ShellExtension = ShellExtension::csharp(
        "KeyValue",
        include_dir!("$CARGO_MANIFEST_DIR/shell/csharp/in_memory"),
    )
    .platforms(&[Platform::DotNet])
    .protocol(&super::PROTOCOL)
    // The names the shipped sources declare at module scope.
    .foreign_types(&["InMemoryKeyValueHandler"]);
}
