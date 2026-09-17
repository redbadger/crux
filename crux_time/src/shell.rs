//! The shell handler `crux_time` ships.
//!
//! The rules a timer follows belong to this crate — that `NotifyAfter` is
//! answered with the timer's own id when it fires, that `Clear` cancels it and
//! answers with the same id, and that an answer arriving after a `Clear` is
//! harmless because the core has stopped listening — so this crate writes them,
//! once per language, and an app asks for them where it configures type
//! generation:
//!
//! ```rust,ignore
//! // shared/src/bin/codegen.rs
//! let typegen = TypeRegistry::new()
//!     .register_app::<MyApp>()?
//!     .shell_handler(&crux_time::TIME)?
//!     .build()?;
//! ```
//!
//! The shell then holds an instance of the emitted type and delegates to it,
//! one line per operation. Nothing is emitted for an app that does not ask.

use crux_core::{
    capability::Operation,
    type_generation::facet::{ShellHandler, ShellSource, TypeGenError, TypeRegistry},
};

use crate::operation::{Clear, NotifyAfter, NotifyAt, Now};

/// What `crux_time` ships for the shell.
///
/// The Swift, Kotlin, TypeScript and C# implementations of the crate's
/// protocol, for an app to register with
/// [`TypeRegistry::shell_handler`](crux_core::type_generation::facet::TypeRegistry::shell_handler).
///
/// Each declares a `TimeHandler` protocol (`ITimeHandler` in C#) with one
/// method per operation, and an implementation of it over the platform's own
/// timers: a sleeping `Task`, a `delay`ing coroutine, `setTimeout` and
/// `Task.Delay`. The timer table is state, so the app constructs one where it
/// constructs its handler.
pub static TIME: ShellHandler = ShellHandler::new("Time")
    .types(register_types)
    .swift(ShellSource::stdlib(include_str!(
        "../shell/swift/Time.swift"
    )))
    .kotlin(ShellSource::stdlib(include_str!("../shell/kotlin/Time.kt")))
    .typescript(ShellSource::stdlib(include_str!(
        "../shell/typescript/time.ts"
    )))
    .csharp(ShellSource::stdlib(include_str!("../shell/csharp/Time.cs")));

/// Registers the types [`TIME`]'s sources name, for an app that does not use
/// every one of them itself.
///
/// A shipped source implements the whole capability, so every type it mentions
/// has to be generated — including for the operations this app never sends. An
/// app that only debounces with `NotifyAfter` and `Clear` still has `Now` and
/// `NotifyAt` in its generated module, because the shipped `TimeHandler`
/// implements them.
fn register_types(registry: &mut TypeRegistry) -> Result<&mut TypeRegistry, TypeGenError> {
    Now::register_types_facet(registry)?;
    NotifyAt::register_types_facet(registry)?;
    NotifyAfter::register_types_facet(registry)?;
    Clear::register_types_facet(registry)
}
