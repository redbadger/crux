//! The shell handler `crux_time` ships, as shell extensions.
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
//!     .shell_extension(&crux_time::shell::swift::TASK)?
//!     .shell_extension(&crux_time::shell::kotlin::COROUTINE)?
//!     .shell_extension(&crux_time::shell::typescript::TIMEOUT)?
//!     .shell_extension(&crux_time::shell::csharp::TASK)?
//!     .build()?;
//! ```
//!
//! The shell then holds an instance of the emitted type and delegates to it,
//! one line per operation. Nothing is emitted for an app that does not ask.
//!
//! The Swift, Kotlin, TypeScript and C# implementations of the crate's
//! protocol each declare a `TimeHandler` protocol (`ITimeHandler` in C#) with
//! one method per operation, and an implementation of it over the platform's
//! own timers: a sleeping `Task`, a `delay`ing coroutine, `setTimeout` and
//! `Task.Delay`. The timer table is state, so the app constructs one where it
//! constructs its handler.

use crux_core::{
    capability::Operation,
    type_generation::facet::{TypeGenError, TypeRegistry},
};

use crate::operation::{Clear, NotifyAfter, NotifyAt, Now};

/// The Swift handler, for an app to register with
/// [`TypeRegistry::shell_extension`](crux_core::type_generation::facet::TypeRegistry::shell_extension).
pub mod swift {
    use crux_core::type_generation::facet::{Platform, ShellExtension};
    use include_dir::include_dir;

    /// `TimeHandler`, and `TaskTimeHandler` over a sleeping `Task`.
    pub static TASK: ShellExtension =
        ShellExtension::swift("Time", include_dir!("$CARGO_MANIFEST_DIR/shell/swift/task"))
            .platforms(&[Platform::Apple, Platform::Linux])
            .rust_types(super::register_types)
            // The names the shipped sources declare at module scope.
            .foreign_types(&["TimeHandler", "TaskTimeHandler"]);
}

/// The Kotlin handler, for an app to register with
/// [`TypeRegistry::shell_extension`](crux_core::type_generation::facet::TypeRegistry::shell_extension).
pub mod kotlin {
    use crux_core::type_generation::facet::{Platform, ShellExtension};
    use include_dir::include_dir;

    /// `TimeHandler`, and `CoroutineTimeHandler` over a `delay`ing coroutine.
    pub static COROUTINE: ShellExtension = ShellExtension::kotlin(
        "Time",
        include_dir!("$CARGO_MANIFEST_DIR/shell/kotlin/coroutine"),
    )
    .platforms(&[Platform::Jvm, Platform::Android])
    .rust_types(super::register_types)
    // The names the shipped sources declare at module scope.
    .foreign_types(&["TimeHandler", "CoroutineTimeHandler"]);
}

/// The TypeScript handler, for an app to register with
/// [`TypeRegistry::shell_extension`](crux_core::type_generation::facet::TypeRegistry::shell_extension).
pub mod typescript {
    use crux_core::type_generation::facet::{Platform, ShellExtension};
    use include_dir::include_dir;

    /// `TimeHandler`, and `TimeoutTimeHandler` over `setTimeout`.
    pub static TIMEOUT: ShellExtension = ShellExtension::typescript(
        "Time",
        include_dir!("$CARGO_MANIFEST_DIR/shell/typescript/timeout"),
    )
    .platforms(&[Platform::Web])
    .rust_types(super::register_types)
    // The names the shipped sources declare at module scope.
    .foreign_types(&["TimeHandler", "MAX_TIMEOUT", "TimeoutTimeHandler"]);
}

/// The C# handler, for an app to register with
/// [`TypeRegistry::shell_extension`](crux_core::type_generation::facet::TypeRegistry::shell_extension).
pub mod csharp {
    use crux_core::type_generation::facet::{Platform, ShellExtension};
    use include_dir::include_dir;

    /// `ITimeHandler`, and `TaskTimeHandler` over `Task.Delay`.
    pub static TASK: ShellExtension = ShellExtension::csharp(
        "Time",
        include_dir!("$CARGO_MANIFEST_DIR/shell/csharp/task"),
    )
    .platforms(&[Platform::DotNet])
    .rust_types(super::register_types)
    // The names the shipped sources declare at module scope.
    .foreign_types(&["ITimeHandler", "TaskTimeHandler"]);
}

/// Registers the types the shipped sources name, for an app that does not use
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
