//! The shell handler `crux_kv` ships.
//!
//! The rules for answering a store operation belong to this crate — that a
//! missing key is a [`Value::None`] and not an error, that `SetValue` and
//! `DeleteValue` answer with what they replaced — so this crate writes them,
//! once per language, and an app asks for them where it configures type
//! generation:
//!
//! ```rust,ignore
//! // shared/src/bin/codegen.rs
//! let typegen = TypeRegistry::new()
//!     .register_app::<MyApp>()?
//!     .shell_handler(&crux_kv::KEY_VALUE)?
//!     .build()?;
//! ```
//!
//! The shell then holds an instance of the emitted type and delegates to it,
//! one line per operation. Nothing is emitted for an app that does not ask.
//!
//! [`Value::None`]: crate::protocol::Value::None

use crux_core::{
    capability::Operation,
    type_generation::facet::{ShellHandler, ShellSource, TypeGenError, TypeRegistry},
};

use crate::operation::{DeleteValue, GetValue, KeyExists, ListKeys, SetValue};

/// What `crux_kv` ships for the shell.
///
/// The Swift, Kotlin, TypeScript and C# implementations of the crate's
/// protocol, for an app to register with
/// [`TypeRegistry::shell_handler`](crux_core::type_generation::facet::TypeRegistry::shell_handler).
///
/// Each declares a `KeyValueHandler` protocol (`IKeyValueHandler` in C#) with
/// one method per operation, and an implementation of it over a store every
/// platform already has: `UserDefaults` on Apple platforms, a file per key on
/// the JVM and in C#, and `localStorage` in the browser. Where the store lives
/// is the shell's choice — each implementation takes it as configuration, and a
/// shell that wants a different one conforms its own type to the protocol.
pub static KEY_VALUE: ShellHandler = ShellHandler::new("KeyValue")
    .types(register_types)
    .swift(ShellSource::stdlib(include_str!(
        "../shell/swift/KeyValue.swift"
    )))
    .kotlin(ShellSource::stdlib(include_str!(
        "../shell/kotlin/KeyValue.kt"
    )))
    .typescript(ShellSource::stdlib(include_str!(
        "../shell/typescript/keyValue.ts"
    )))
    .csharp(ShellSource::stdlib(include_str!(
        "../shell/csharp/KeyValue.cs"
    )));

/// Registers the types [`KEY_VALUE`]'s sources name, for an app that does not
/// use every one of them itself.
///
/// A shipped source implements the whole capability, so every type it mentions
/// has to be generated — including for the operations this app never sends. An
/// app that only reads and writes still has `DeleteValue`, `KeyExists` and
/// `ListKeys` in its generated module, because the shipped `KeyValueHandler`
/// implements them.
///
/// Those names are therefore taken in the app's module whether or not the app
/// sends the operations, and type generation rejects an app type that
/// generates under one of them.
fn register_types(registry: &mut TypeRegistry) -> Result<&mut TypeRegistry, TypeGenError> {
    GetValue::register_types_facet(registry)?;
    SetValue::register_types_facet(registry)?;
    DeleteValue::register_types_facet(registry)?;
    KeyExists::register_types_facet(registry)?;
    ListKeys::register_types_facet(registry)
}
