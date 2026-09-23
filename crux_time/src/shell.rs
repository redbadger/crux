//! The shell handler `crux_time` ships, as a shell extension.
//!
//! The rules a timer follows belong to this crate — that `NotifyAfter` is
//! answered with the timer's own id when it fires, that `ClearTimer` cancels it
//! and answers with the same id, and that an answer arriving after a
//! `ClearTimer` is harmless because the core has stopped listening — so this
//! crate writes them, once per language, and an app asks for them where it
//! configures type generation:
//!
//! ```rust,ignore
//! // shared/src/bin/codegen.rs
//! let typegen = TypeRegistry::new()
//!     .register_app::<MyApp>()?
//!     .shell_extension(&crux_time::shell::EXTENSION)?
//!     .build()?;
//! ```
//!
//! The shell then holds an instance of the emitted type and delegates to it,
//! one line per operation. Nothing is emitted for an app that does not ask.

use crux_core::{
    capability::Operation,
    type_generation::facet::{ShellExtension, ShellSource, TypeGenError, TypeRegistry},
};

use crate::operation::{ClearTimer, NotifyAfter, NotifyAt, Now};

/// What `crux_time` ships for the shell.
///
/// The Swift, Kotlin, TypeScript and C# implementations of the crate's
/// protocol, for an app to register with
/// [`TypeRegistry::shell_extension`](crux_core::type_generation::facet::TypeRegistry::shell_extension).
///
/// Each declares a `TimeHandler` protocol (`ITimeHandler` in C#) with one
/// method per operation, and an implementation of it over the platform's own
/// timers: a sleeping `Task`, a `delay`ing coroutine, `setTimeout` and
/// `Task.Delay`. The timer table is state, so the app constructs one where it
/// constructs its handler.
pub static EXTENSION: ShellExtension = ShellExtension::new("Time")
    .rust_types(register_types)
    // The names the shipped sources declare at module scope, in any language.
    .foreign_types(&[
        "TimeHandler",
        "ITimeHandler",
        "TaskTimeHandler",
        "CoroutineTimeHandler",
        "TimeoutTimeHandler",
        "MAX_TIMEOUT",
    ])
    .swift(ShellSource::stdlib(include_str!(
        "../shell/swift/Time.swift"
    )))
    .kotlin(ShellSource::stdlib(include_str!("../shell/kotlin/Time.kt")))
    .typescript(ShellSource::stdlib(include_str!(
        "../shell/typescript/time.ts"
    )))
    .csharp(ShellSource::stdlib(include_str!("../shell/csharp/Time.cs")));

/// Registers the types [`EXTENSION`]'s sources name, for an app that does not use
/// every one of them itself.
///
/// A shipped source implements the whole capability, so every type it mentions
/// has to be generated — including for the operations this app never sends. An
/// app that only debounces with `NotifyAfter` and `ClearTimer` still has `Now`
/// and `NotifyAt` in its generated module, because the shipped `TimeHandler`
/// implements them.
fn register_types(registry: &mut TypeRegistry) -> Result<&mut TypeRegistry, TypeGenError> {
    Now::register_types_facet(registry)?;
    NotifyAt::register_types_facet(registry)?;
    NotifyAfter::register_types_facet(registry)?;
    ClearTimer::register_types_facet(registry)
}
