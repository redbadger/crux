//! The shipped shell sources, compiled against the types they are emitted
//! beside.
//!
//! `cargo test` cannot compile Swift, Kotlin, TypeScript or C#, so a shipped
//! file that does not build is caught either here or in an example shell — and
//! only an example that registers the handler covers it. This test registers
//! `crux_time::TIME` against a small app and hands the generated package to
//! whichever toolchain is on `PATH`: a Rust contributor without the .NET SDK
//! sees a skip, and CI, which has one, sees a compile. Set
//! `CRUX_REQUIRE_SHELL_TOOLCHAINS` — `just ci` and the `build` workflow both
//! do — and a toolchain that is not there fails the test instead.
#![cfg(feature = "facet_typegen")]
// `#[derive(Facet)]` generates `unsafe` methods.
#![allow(clippy::unsafe_derive_deserialize)]

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use crux_core::{
    Command as CruxCommand,
    macros::effect,
    render::RenderOperation,
    type_generation::facet::{CodeGenerator, Config, TypeRegistry},
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

fn generator() -> CodeGenerator {
    let mut registry = TypeRegistry::new();
    registry
        .register_app::<App>()
        .expect("should register the app");
    // Registering the handler registers the types its sources name, which for
    // this app includes the operations it never sends.
    registry
        .shell_handler(&crux_time::TIME)
        .expect("should register the shipped handler");
    registry.build().expect("should build the registry")
}

/// Whether a toolchain this test cannot find is a failure rather than a skip.
///
/// `just ci` and the `build` workflow both set
/// `CRUX_REQUIRE_SHELL_TOOLCHAINS`, because there every toolchain is installed
/// and a skip would mean a shipped source went unbuilt with nobody told. Unset
/// — a Rust contributor without Swift, Kotlin, Node or the .NET SDK still gets
/// a green run.
fn toolchains_required() -> bool {
    std::env::var_os("CRUX_REQUIRE_SHELL_TOOLCHAINS").is_some_and(|value| !value.is_empty())
}

/// Reports a toolchain this test needs and did not find: a failure when
/// `CRUX_REQUIRE_SHELL_TOOLCHAINS` is set, and a printed skip otherwise.
fn unavailable(missing: &str, consequence: &str) {
    assert!(
        !toolchains_required(),
        "{missing}, so {consequence}. CRUX_REQUIRE_SHELL_TOOLCHAINS is set, so a missing \
         toolchain fails rather than skips: install it, or unset the variable."
    );
    println!("skipping: {missing}, so {consequence}");
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
            unavailable(
                "`swift` is not on PATH",
                "the shipped Swift is not compiled",
            );
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
            unavailable("`dotnet` is not on PATH", "the shipped C# is not compiled");
        }
        Err(e) => panic!("could not run `dotnet build`: {e}"),
    }
}

#[test]
fn the_swift_source_compiles() {
    let dir = tempfile::tempdir().expect("should create a temp dir");
    generator()
        .swift(&Config::builder("App", dir.path()).build())
        .expect("swift type generation should succeed");

    let shipped = fs::read_to_string(dir.path().join("App/Sources/App/Time.swift"))
        .expect("should write the handler beside the module");
    assert!(shipped.contains("public protocol TimeHandler: Sendable {"));
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
    generator()
        .kotlin(&Config::builder("com.example.shared", dir.path()).build())
        .expect("kotlin type generation should succeed");

    let shipped = fs::read_to_string(dir.path().join("com/example/shared/Time.kt"))
        .expect("should write the handler beside the module");
    assert!(shipped.starts_with("package com.example.shared\n"));
    assert!(shipped.contains("interface TimeHandler {"));
    assert!(shipped.contains("class CoroutineTimeHandler("));
}

/// `typescript()` runs `tsc` over what it wrote, so generating at all is a
/// compile of the shipped source against the types above it.
#[test]
fn the_typescript_source_compiles() {
    let dir = tempfile::tempdir().expect("should create a temp dir");
    generator()
        .typescript(&Config::builder("shared_types", dir.path()).build())
        .expect("typescript type generation should succeed");

    let declarations = fs::read_to_string(dir.path().join("shared_types.d.ts"))
        .expect("tsc should have emitted declarations");
    assert!(declarations.contains("TimeoutTimeHandler"));
}

#[test]
fn the_csharp_source_compiles() {
    let dir = tempfile::tempdir().expect("should create a temp dir");
    generator()
        .csharp(&Config::builder("Example.Shared", dir.path()).build())
        .expect("c# type generation should succeed");

    let shipped = fs::read_to_string(dir.path().join("Example/Shared/Time.cs"))
        .expect("should write the handler beside the module");
    assert!(shipped.contains("public interface ITimeHandler"));

    dotnet_build(dir.path());
}

// ---------------------------------------------------------------------------
// The shipped shell sources, run.
//
// Compiling a handler says only that it type-checks against the module it is
// emitted beside, and a timer is all behaviour: which id it answers with, when
// it answers, and whether a cleared one settles at all. None of that is a
// compile error in any language, and the last of them is a handler that hangs
// rather than one that fails.
//
// So each of these tests generates the package, writes a harness project of
// its own beside it — checked in under `tests/harness/`, and not something
// type generation emits — and runs it. The harness drives the handler through
// the whole protocol and prints `HARNESS OK` when every answer was the one
// `crux_time::operation` promises.
// ---------------------------------------------------------------------------

/// Writes the harness files this crate ships into `dir`.
fn write_harness(dir: &Path, files: &[(&str, &str)]) {
    for (name, contents) in files {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("should create the harness directory");
        }
        fs::write(&path, contents).expect("should write the harness file");
    }
}

/// Runs `program` in `dir`, or says why it did not.
///
/// A missing toolchain is a skip, exactly as the compile tests above skip, so
/// that a contributor without a Swift, .NET, Kotlin or Node install still gets
/// a green run — unless `CRUX_REQUIRE_SHELL_TOOLCHAINS` is set, where it is a
/// failure naming what is missing.
fn run(program: &str, args: &[&str], dir: &Path) -> Option<Output> {
    match Command::new(program).current_dir(dir).args(args).output() {
        Ok(output) => Some(output),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            unavailable(
                &format!("`{program}` is not on PATH"),
                "the shipped source is not run",
            );
            None
        }
        Err(e) => panic!("could not run `{program}`: {e}"),
    }
}

/// Asserts that a build step succeeded, with everything it printed.
fn succeeded(program: &str, output: &Output) {
    assert!(
        output.status.success(),
        "`{program}` failed:\n{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Asserts that a harness ran and passed, with the line it printed for every
/// answer that was not the one the protocol promises.
///
/// The last line has to be `HARNESS OK`: an exit code of zero is not enough
/// when a runtime can drop an unsettled promise and exit zero with nothing to
/// show for it.
fn passed(program: &str, output: &Output) {
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains("HARNESS OK"),
        "the `{program}` harness did not pass:\n{stdout}{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// The `kotlinx-coroutines-core` jar the generated module needs, from wherever
/// a build tool has already put one.
///
/// `kotlinc` ships the standard library and nothing else, while the generated
/// module imports `kotlinx.coroutines`, so the harness needs the jar on its
/// classpath. Gradle and Maven each keep one under the home directory once
/// anything has built against it — the weather example's Android shell has —
/// and fetching one here would mean a build tool and a network. Set
/// `KOTLINX_COROUTINES_JAR` to point at one elsewhere — the `build` workflow
/// downloads one from Maven Central and does exactly that.
fn coroutines_jar() -> Option<PathBuf> {
    if let Ok(jar) = std::env::var("KOTLINX_COROUTINES_JAR") {
        return Some(PathBuf::from(jar));
    }

    let home = PathBuf::from(std::env::var("HOME").ok()?);
    [
        home.join(
            ".gradle/caches/modules-2/files-2.1/org.jetbrains.kotlinx/kotlinx-coroutines-core-jvm",
        ),
        home.join(".m2/repository/org/jetbrains/kotlinx/kotlinx-coroutines-core-jvm"),
    ]
    .iter()
    .find_map(|root| find_jar(root, 4))
}

/// The first jar under `dir`, no deeper than `depth`, ignoring the `-sources`
/// jars that sit beside the real ones.
fn find_jar(dir: &Path, depth: usize) -> Option<PathBuf> {
    if depth == 0 {
        return None;
    }

    let mut entries: Vec<_> = fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect();
    entries.sort();

    entries.iter().find_map(|path| {
        if path.is_dir() {
            return find_jar(path, depth - 1);
        }

        let is_jar = path.extension().is_some_and(|extension| extension == "jar");
        let is_sources = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .is_some_and(|stem| stem.ends_with("-sources"));

        (is_jar && !is_sources).then(|| path.clone())
    })
}

#[test]
fn the_swift_source_behaves() {
    let dir = tempfile::tempdir().expect("should create a temp dir");
    generator()
        .swift(&Config::builder("App", dir.path().join("generated")).build())
        .expect("swift type generation should succeed");

    let harness = dir.path().join("harness");
    write_harness(
        &harness,
        &[
            ("Package.swift", include_str!("harness/Package.swift")),
            (
                "Sources/Harness/main.swift",
                include_str!("harness/main.swift"),
            ),
        ],
    );

    if let Some(output) = run("swift", &["run"], &harness) {
        passed("swift", &output);
    }
}

/// The weather example's Android shell is where the shipped Kotlin is built on
/// every change, but only running it says the timer answers at all. The
/// `build` workflow installs `kotlinc` and a coroutines jar so that this runs
/// in CI as well as locally.
#[test]
fn the_kotlin_source_behaves() {
    let dir = tempfile::tempdir().expect("should create a temp dir");
    generator()
        .kotlin(&Config::builder("com.example.shared", dir.path().join("generated")).build())
        .expect("kotlin type generation should succeed");

    let harness = dir.path().join("harness");
    write_harness(&harness, &[("Main.kt", include_str!("harness/Main.kt"))]);

    let Some(coroutines) = coroutines_jar() else {
        unavailable(
            "there is no kotlinx-coroutines-core jar in KOTLINX_COROUTINES_JAR or under \
             ~/.gradle or ~/.m2, and `kotlinc` ships only the standard library",
            "the shipped Kotlin is not run",
        );
        return;
    };

    let classpath = coroutines.to_string_lossy().into_owned();
    // The generated package's own directory, not its root, because `kotlinc`
    // would otherwise try to compile the `build.gradle.kts` beside it.
    let compile = [
        "../generated/com",
        "Main.kt",
        "-classpath",
        &classpath,
        "-d",
        "harness.jar",
    ];
    let Some(compiled) = run("kotlinc", &compile, &harness) else {
        return;
    };
    succeeded("kotlinc", &compiled);

    let separator = if cfg!(windows) { ';' } else { ':' };
    let classpath = format!("harness.jar{separator}{classpath}");
    if let Some(output) = run("kotlin", &["-classpath", &classpath, "MainKt"], &harness) {
        passed("kotlin", &output);
    }
}

#[test]
fn the_typescript_source_behaves() {
    let dir = tempfile::tempdir().expect("should create a temp dir");
    generator()
        .typescript(&Config::builder("shared_types", dir.path().join("generated")).build())
        .expect("typescript type generation should succeed");

    let harness = dir.path().join("harness");
    write_harness(&harness, &[("main.js", include_str!("harness/main.js"))]);

    if let Some(output) = run("node", &["main.js"], &harness) {
        passed("node", &output);
    }
}

#[test]
fn the_csharp_source_behaves() {
    let dir = tempfile::tempdir().expect("should create a temp dir");
    generator()
        .csharp(&Config::builder("Example.Shared", dir.path().join("generated")).build())
        .expect("c# type generation should succeed");

    let harness = dir.path().join("harness");
    write_harness(
        &harness,
        &[
            ("Harness.csproj", include_str!("harness/Harness.csproj")),
            ("Program.cs", include_str!("harness/Program.cs")),
        ],
    );

    if let Some(output) = run("dotnet", &["run", "--nologo"], &harness) {
        passed("dotnet", &output);
    }
}
