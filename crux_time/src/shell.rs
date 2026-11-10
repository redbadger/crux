//! The shell protocol of `crux_time`.
//!
//! The protocol is the interface that a shell implements for this capability:
//! `TimeHandler` (`ITimeHandler` in C#), with one method for each operation.
//! Type generation emits it in Swift, Kotlin, TypeScript and C# for every app
//! that uses an operation of `crux_time`. The app does not register it. Each
//! operation names it with `protocol = crate::shell::PROTOCOL` in its
//! `#[operation(..)]` attribute.
//!
//! The protocol follows the `facet_typegen` feature of `crux_core`. The items
//! of this module, and the part of each operation that registers the
//! protocol, exist only when that feature is on. Thus an app that turns on
//! `crux_core/facet_typegen` for type generation gets the protocol, and needs
//! no feature of `crux_time` for it.
//!
//! A one-shot timer is answered exactly once, with its own id. A repeating
//! timer (`NotifyEvery`) is a stream: the shell sends its id once per period,
//! until a `ClearTimer` for that id arrives. `ClearTimer` is answered with the
//! id of the timer that it cancels. The protocol states these rules in each
//! language.
//!
//! # Implementations
//!
//! The crate also ships an implementation of the protocol in each language,
//! over the platform's own timers: `TaskTimeHandler` over a sleeping `Task`
//! in Swift, `CoroutineTimeHandler` over a coroutine that calls `delay` in
//! Kotlin, `TimeoutTimeHandler` over `setTimeout` in TypeScript, and
//! `TaskTimeHandler` over `Task.Delay` in C#. An implementation is a shell
//! extension, and an app gets one only if it registers it where it configures
//! type generation:
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
//! The timer table is state, so the shell constructs one instance of the
//! emitted type where it constructs its handler, and delegates each operation
//! to it in one line.
//!
//! Each implementation computes the deadlines of a repeating timer from a
//! monotonic start time, so the timer does not drift. After a late wake, it
//! sends one tick, not one for each period that it missed. It checks for a
//! `clear` before each tick. A tick that started before the `clear` can still
//! reach the core after it, and the core ignores that tick. The shell cannot
//! tell when the app aborts the command without a `ClearTimer`. The timer then
//! continues, and the core ignores its ticks.
//!
//! The implementations need the `facet_typegen` feature of `crux_time`.

crux_core::__crux_core_facet_typegen_items! {
    use crux_core::{
        capability::Operation as _,
        type_generation::facet::{ShellProtocol, TypeGenError, TypeRegistry},
    };

    use crate::operation::{ClearTimer, NotifyAfter, NotifyAt, NotifyEvery, Now};

    /// The protocol of `crux_time`, in each of the four languages.
    pub static PROTOCOL: ShellProtocol = ShellProtocol::new("Time")
        .rust_types(register_types)
        .swift(include_str!("../shell/swift/Time.swift"), &["TimeHandler"])
        .kotlin(include_str!("../shell/kotlin/Time.kt"), &["TimeHandler"])
        .typescript(
            include_str!("../shell/typescript/time.ts"),
            &["TimeHandler"],
        )
        .csharp(include_str!("../shell/csharp/Time.cs"), &["ITimeHandler"]);

    /// Registers every operation of `crux_time` and its output.
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
        Now::register_types_facet(registry)?;
        NotifyAt::register_types_facet(registry)?;
        NotifyAfter::register_types_facet(registry)?;
        NotifyEvery::register_types_facet(registry)?;
        ClearTimer::register_types_facet(registry)
    }
}

/// The Swift implementation, for an app to register with
/// [`TypeRegistry::shell_extension`](crux_core::type_generation::facet::TypeRegistry::shell_extension).
#[cfg(feature = "facet_typegen")]
pub mod swift {
    use crux_core::type_generation::facet::{Platform, ShellExtension};
    use include_dir::include_dir;

    /// `TaskTimeHandler`, over a sleeping `Task`.
    pub static TASK: ShellExtension =
        ShellExtension::swift("Time", include_dir!("$CARGO_MANIFEST_DIR/shell/swift/task"))
            .platforms(&[Platform::Apple, Platform::Linux])
            .protocol(&super::PROTOCOL)
            // The names the shipped sources declare at module scope.
            .foreign_types(&["TaskTimeHandler"]);
}

/// The Kotlin implementation, for an app to register with
/// [`TypeRegistry::shell_extension`](crux_core::type_generation::facet::TypeRegistry::shell_extension).
#[cfg(feature = "facet_typegen")]
pub mod kotlin {
    use crux_core::type_generation::facet::{Platform, ShellExtension};
    use include_dir::include_dir;

    /// `CoroutineTimeHandler`, over a coroutine that calls `delay`.
    pub static COROUTINE: ShellExtension = ShellExtension::kotlin(
        "Time",
        include_dir!("$CARGO_MANIFEST_DIR/shell/kotlin/coroutine"),
    )
    .platforms(&[Platform::Jvm, Platform::Android])
    .protocol(&super::PROTOCOL)
    // The names the shipped sources declare at module scope.
    .foreign_types(&["CoroutineTimeHandler"]);
}

/// The TypeScript implementation, for an app to register with
/// [`TypeRegistry::shell_extension`](crux_core::type_generation::facet::TypeRegistry::shell_extension).
#[cfg(feature = "facet_typegen")]
pub mod typescript {
    use crux_core::type_generation::facet::{Platform, ShellExtension};
    use include_dir::include_dir;

    /// `TimeoutTimeHandler`, over `setTimeout`.
    pub static TIMEOUT: ShellExtension = ShellExtension::typescript(
        "Time",
        include_dir!("$CARGO_MANIFEST_DIR/shell/typescript/timeout"),
    )
    .platforms(&[Platform::Web])
    .protocol(&super::PROTOCOL)
    // The names the shipped sources declare at module scope.
    .foreign_types(&["MAX_TIMEOUT", "TimeoutTimeHandler"]);
}

/// The C# implementation, for an app to register with
/// [`TypeRegistry::shell_extension`](crux_core::type_generation::facet::TypeRegistry::shell_extension).
#[cfg(feature = "facet_typegen")]
pub mod csharp {
    use crux_core::type_generation::facet::{Platform, ShellExtension};
    use include_dir::include_dir;

    /// `TaskTimeHandler`, over `Task.Delay`.
    pub static TASK: ShellExtension = ShellExtension::csharp(
        "Time",
        include_dir!("$CARGO_MANIFEST_DIR/shell/csharp/task"),
    )
    .platforms(&[Platform::DotNet])
    .protocol(&super::PROTOCOL)
    // The names the shipped sources declare at module scope.
    .foreign_types(&["TaskTimeHandler"]);
}
