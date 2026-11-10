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
