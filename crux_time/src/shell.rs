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
//! A timer is answered exactly once, with its own id, and `ClearTimer` is
//! answered with the id of the timer that it cancels. The protocol states
//! these rules in each language.

crux_core::__crux_core_facet_typegen_items! {
    use crux_core::{
        capability::Operation as _,
        type_generation::facet::{ShellProtocol, TypeGenError, TypeRegistry},
    };

    use crate::operation::{ClearTimer, NotifyAfter, NotifyAt, Now};

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
    /// module whether or not the app sends the operations. Do not give an app
    /// type one of these names.
    ///
    /// # Errors
    /// Returns a [`TypeGenError`] if a type cannot be registered.
    pub fn register_types(registry: &mut TypeRegistry) -> Result<&mut TypeRegistry, TypeGenError> {
        Now::register_types_facet(registry)?;
        NotifyAt::register_types_facet(registry)?;
        NotifyAfter::register_types_facet(registry)?;
        ClearTimer::register_types_facet(registry)
    }
}
