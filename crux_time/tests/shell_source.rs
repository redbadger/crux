//! The shipped shell sources, compiled against the types they are emitted
//! beside.
//!
//! `cargo test` cannot compile Swift, Kotlin, TypeScript or C#, so a shipped
//! file that does not build is caught either here or in an example shell — and
//! only an example that registers the handler covers it. This test registers
//! each extension in `crux_time::shell` against a small app and hands the generated package to
//! whichever toolchain is on `PATH`: a Rust contributor without the .NET SDK
//! sees a skip, and CI, which has one, sees a compile.
#![cfg(feature = "facet_typegen")]
// `#[derive(Facet)]` generates `unsafe` methods.
#![allow(clippy::unsafe_derive_deserialize)]

use std::{fs, path::Path, process::Command};

use crux_core::{
    Command as CruxCommand,
    macros::effect,
    render::RenderOperation,
    type_generation::facet::{CodeGenerator, Config, ShellExtension, TypeRegistry},
};
use crux_time::operation::{ClearTimer, NotifyAfter, NotifyAt, Now};
use facet::Facet;

#[derive(Facet)]
#[repr(C)]
pub enum Event {
    None,
}

#[derive(Facet)]
pub struct ViewModel;

/// The smallest app that gives the shipped source something to implement.
#[effect(facet_typegen)]
pub enum Effect {
    Render(RenderOperation),
    TimeNow(Now),
    TimeNotifyAt(NotifyAt),
    TimeNotifyAfter(NotifyAfter),
    TimeClear(ClearTimer),
}

#[derive(Default)]
pub struct App;

impl crux_core::App for App {
    type Event = Event;
    type Model = ();
    type ViewModel = ViewModel;
    type Effect = Effect;

    fn update(&self, _event: Event, _model: &mut Self::Model) -> CruxCommand<Effect, Event> {
        CruxCommand::done()
    }

    fn view(&self, _model: &Self::Model) -> Self::ViewModel {
        ViewModel
    }
}

fn generator(extension: &'static ShellExtension) -> CodeGenerator {
    let mut registry = TypeRegistry::new();
    registry
        .register_app::<App>()
        .expect("should register the app");
    // The app's operation registers the protocol. The extension registers
    // only its implementation, beside the protocol.
    registry
        .shell_extension(extension)
        .expect("should register the shipped handler");
    registry.build().expect("should build the registry")
}

/// Builds a generated Swift package, or says why it did not.
fn swift_build(dir: &Path) {
    match Command::new("swift").current_dir(dir).arg("build").output() {
        Ok(output) => assert!(
            output.status.success(),
            "`swift build` failed:\n{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            println!("skipping: `swift` is not on PATH, so the shipped Swift is not compiled");
        }
        Err(e) => panic!("could not run `swift build`: {e}"),
    }
}

/// Builds a generated C# package, or says why it did not.
fn dotnet_build(dir: &Path) {
    match Command::new("dotnet")
        .current_dir(dir)
        .args(["build", "--nologo"])
        .output()
    {
        Ok(output) => assert!(
            output.status.success(),
            "`dotnet build` failed:\n{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            println!("skipping: `dotnet` is not on PATH, so the shipped C# is not compiled");
        }
        Err(e) => panic!("could not run `dotnet build`: {e}"),
    }
}

#[test]
fn the_swift_source_compiles() {
    let dir = tempfile::tempdir().expect("should create a temp dir");
    generator(&crux_time::shell::swift::TASK)
        .swift(&Config::builder("App", dir.path()).build())
        .expect("swift type generation should succeed");

    let sources = dir.path().join("App/Sources/App");
    let protocol = fs::read_to_string(sources.join("Time.swift"))
        .expect("should write the protocol beside the module");
    assert!(protocol.contains("public protocol TimeHandler: Sendable {"));
    let shipped = fs::read_to_string(sources.join("TaskTime.swift"))
        .expect("should write the handler beside the module");
    assert!(
        shipped.contains("public final class TaskTimeHandler: TimeHandler, @unchecked Sendable {")
    );

    swift_build(&dir.path().join("App"));
}

/// Kotlin has no compiler this test can count on — the weather example's
/// Android shell is where the shipped source is built — so this only checks
/// that it lands where the app will find it, under the module's package.
#[test]
fn the_kotlin_source_is_emitted() {
    let dir = tempfile::tempdir().expect("should create a temp dir");
    generator(&crux_time::shell::kotlin::COROUTINE)
        .kotlin(&Config::builder("com.example.shared", dir.path()).build())
        .expect("kotlin type generation should succeed");

    let package = dir.path().join("com/example/shared");
    let protocol = fs::read_to_string(package.join("Time.kt"))
        .expect("should write the protocol beside the module");
    assert!(protocol.contains("interface TimeHandler {"));
    let shipped = fs::read_to_string(package.join("CoroutineTime.kt"))
        .expect("should write the handler beside the module");
    assert!(shipped.starts_with("package com.example.shared\n"));
    assert!(shipped.contains("class CoroutineTimeHandler("));
}

/// `typescript()` runs `tsc` over what it wrote, so generating at all is a
/// compile of the shipped source against the types above it.
#[test]
fn the_typescript_source_compiles() {
    let dir = tempfile::tempdir().expect("should create a temp dir");
    generator(&crux_time::shell::typescript::TIMEOUT)
        .typescript(&Config::builder("shared_types", dir.path()).build())
        .expect("typescript type generation should succeed");

    let declarations = fs::read_to_string(dir.path().join("shared_types.d.ts"))
        .expect("tsc should have emitted declarations");
    assert!(declarations.contains("TimeoutTimeHandler"));
}

#[test]
fn the_csharp_source_compiles() {
    let dir = tempfile::tempdir().expect("should create a temp dir");
    generator(&crux_time::shell::csharp::TASK)
        .csharp(&Config::builder("Example.Shared", dir.path()).build())
        .expect("c# type generation should succeed");

    let protocol = fs::read_to_string(dir.path().join("Example/Shared/Time.cs"))
        .expect("should write the protocol beside the module");
    assert!(protocol.contains("public interface ITimeHandler"));
    let shipped = fs::read_to_string(dir.path().join("Example/Shared/TaskTime.cs"))
        .expect("should write the handler beside the module");
    assert!(shipped.contains("public sealed class TaskTimeHandler : ITimeHandler"));

    dotnet_build(dir.path());
}
