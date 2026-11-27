#[cfg(feature = "facet_typegen")]
mod facet_shared {
    // `#[derive(Facet)]` generates `unsafe` methods.
    #![allow(clippy::unsafe_derive_deserialize)]

    use crux_core::{
        Command,
        macros::{Operation, effect},
        render::RenderOperation,
    };
    use facet::Facet;
    use serde::{Deserialize, Serialize};

    #[derive(Facet)]
    #[repr(C)]
    pub enum Event {
        None,
    }

    #[derive(Facet)]
    pub struct ViewModel;

    #[derive(Operation, Facet, Debug, Clone, Serialize, Deserialize)]
    #[operation(request, output = GetResult)]
    pub struct Get {
        pub key: String,
    }

    #[derive(Operation, Facet, Debug, Clone, Serialize, Deserialize)]
    #[operation(notify)]
    pub struct Publish(pub Vec<u8>);

    #[derive(Operation, Facet, Debug, Clone, Serialize, Deserialize)]
    #[operation(stream, output = Message)]
    pub struct Subscribe;

    /// An operation whose output lives in a namespace of its own, which is what
    /// the generated handler signature has to be able to name from inside the
    /// module's own package.
    #[derive(Operation, Facet, Debug, Clone, Serialize, Deserialize)]
    #[operation(request, output = Presence)]
    #[facet(facet_generate_attrs::namespace = "Kit")]
    pub struct Probe;

    #[derive(Facet, Debug, Clone, Serialize, Deserialize)]
    #[repr(C)]
    #[facet(facet_generate_attrs::namespace = "Kit")]
    pub enum Presence {
        Present,
        Absent,
    }

    /// An operation that declares no kind, so the shell resolves it by hand.
    #[derive(Facet, Debug, Clone, Serialize, Deserialize)]
    pub struct Legacy {
        pub topic: String,
    }

    impl crux_core::capability::Operation for Legacy {
        type Output = Message;
    }

    #[derive(Facet, Debug, Clone, Serialize, Deserialize)]
    pub struct Message {
        pub body: Vec<u8>,
    }

    #[derive(Facet, Debug, Clone, Serialize, Deserialize)]
    #[repr(C)]
    pub enum GetResult {
        Ok(Vec<u8>),
        Err(StoreError),
    }

    #[derive(Facet, Debug, Clone, Serialize, Deserialize)]
    #[repr(C)]
    pub enum StoreError {
        NotFound,
    }

    #[effect(facet_typegen)]
    pub enum Effect {
        Render(RenderOperation),
        Get(Get),
        Publish(Publish),
        Subscribe(Subscribe),
        Probe(Probe),
        Legacy(Legacy),
    }

    #[derive(Default)]
    pub struct App;

    impl crux_core::App for App {
        type Event = Event;
        type Model = ();
        type ViewModel = ViewModel;
        type Effect = Effect;

        fn update(&self, _event: Event, _model: &mut Self::Model) -> Command<Effect, Event> {
            Command::done()
        }

        fn view(&self, _model: &Self::Model) -> Self::ViewModel {
            ViewModel
        }
    }

    #[derive(Facet)]
    #[repr(C)]
    pub enum OtherEvent {
        None,
    }

    #[derive(Facet)]
    pub struct OtherViewModel;

    /// A second app over the same effect, for checking that the recorded
    /// [`AppMeta`](crux_core::type_generation::facet::AppMeta) is the first
    /// one's.
    #[derive(Default)]
    pub struct OtherApp;

    impl crux_core::App for OtherApp {
        type Event = OtherEvent;
        type Model = ();
        type ViewModel = OtherViewModel;
        type Effect = Effect;

        fn update(
            &self,
            _event: OtherEvent,
            _model: &mut Self::Model,
        ) -> Command<Effect, OtherEvent> {
            Command::done()
        }

        fn view(&self, _model: &Self::Model) -> Self::ViewModel {
            OtherViewModel
        }
    }
}

#[cfg(feature = "facet_typegen")]
mod facet_test {
    use std::fs;

    use crux_core::{
        OperationKind,
        type_generation::facet::{
            BoltFfi, Config, Format, PackageLocation, TypeGenError, TypeRegistry,
        },
    };

    use super::{
        facet_shared::{App, OtherApp},
        toolchain::{build, kotlinc_build},
    };

    #[test]
    fn register_app_records_event_and_view_model() {
        let mut registry = TypeRegistry::new();
        let generator = registry
            .register_app::<App>()
            .expect("should register the app")
            .build()
            .expect("should build the registry");

        let app = generator.app().expect("the app should be recorded");
        assert_eq!(app.event.name, "Event");
        assert_eq!(app.view_model.name, "ViewModel");

        // The generated `Core` uses fixed names, so the first app wins.
        let generator = registry
            .register_app::<OtherApp>()
            .expect("should register the second app")
            .build()
            .expect("should build the registry");

        let app = generator.app().expect("the app should be recorded");
        assert_eq!(app.event.name, "Event");
        assert_eq!(app.view_model.name, "ViewModel");
    }

    #[test]
    fn a_render_variant_is_recorded() {
        let generator = TypeRegistry::new()
            .register_app::<App>()
            .expect("should register the app")
            .build()
            .expect("should build the registry");

        let flags: Vec<_> = generator.effects()[0]
            .variants
            .iter()
            .map(|variant| (variant.ident.as_str(), variant.render))
            .collect();

        assert_eq!(
            flags,
            vec![
                ("Render", true),
                ("Get", false),
                ("Publish", false),
                ("Subscribe", false),
                ("Probe", false),
                ("Legacy", false),
            ]
        );
    }

    #[test]
    fn effect_variants_carry_their_declared_operation_kind() {
        let generator = TypeRegistry::new()
            .register_app::<App>()
            .expect("should register the app")
            .build()
            .expect("should build the registry");

        let [effect] = generator.effects() else {
            panic!("expected exactly one effect, got {:?}", generator.effects());
        };

        assert_eq!(effect.effect.name, "Effect");

        let kinds: Vec<_> = effect
            .variants
            .iter()
            .map(|variant| (variant.ident.as_str(), variant.kind))
            .collect();

        assert_eq!(
            kinds,
            vec![
                ("Render", Some(OperationKind::Notify)),
                ("Get", Some(OperationKind::Request)),
                ("Publish", Some(OperationKind::Notify)),
                ("Subscribe", Some(OperationKind::Stream)),
                ("Probe", Some(OperationKind::Request)),
                ("Legacy", None),
            ]
        );
    }

    #[test]
    fn a_request_variant_records_the_type_it_resolves_with() {
        let generator = TypeRegistry::new()
            .register_app::<App>()
            .expect("should register the app")
            .build()
            .expect("should build the registry");

        let get = generator.effects()[0]
            .variants
            .iter()
            .find(|variant| variant.ident == "Get")
            .expect("`Get` should be recorded");

        let Some(Format::TypeName(name)) = &get.output else {
            panic!("expected a named output, got {:?}", get.output);
        };
        assert_eq!(name.name, "GetResult");

        // A notification is never resolved, so it records no output.
        let render = generator.effects()[0]
            .variants
            .iter()
            .find(|variant| variant.ident == "Render")
            .expect("`Render` should be recorded");
        assert_eq!(render.output, None);
    }

    /// `StoreError` is only reachable through `GetResult::Err`, and nothing
    /// names it: the registry walks the output's shape and finds it itself.
    #[test]
    fn nested_output_types_are_registered_without_being_named() {
        let registry = TypeRegistry::new()
            .register_app::<App>()
            .expect("should register the app")
            .build()
            .expect("should build the registry")
            .registry();

        for name in ["Get", "GetResult", "StoreError", "Publish", "Effect"] {
            assert!(
                registry.keys().any(|key| key.name == name),
                "expected {name} in the registry, got {:?}",
                registry.keys().collect::<Vec<_>>()
            );
        }
    }

    // -----------------------------------------------------------------------
    // The real code generators, end to end
    // -----------------------------------------------------------------------

    fn generator() -> crux_core::type_generation::facet::CodeGenerator {
        TypeRegistry::new()
            .register_app::<App>()
            .expect("should register the app")
            .build()
            .expect("should build the registry")
    }

    /// Every generated module should carry the operation kinds and the handler
    /// API, whatever the language spells them.
    fn assert_generated(source: &str, expected: &[&str]) {
        for fragment in expected {
            assert!(
                source.contains(fragment),
                "expected the generated source to contain `{fragment}`:\n{source}"
            );
        }
    }

    #[test]
    fn generates_swift() {
        let dir = tempfile::tempdir().expect("should create a temp dir");
        generator()
            .swift(&Config::builder("App", dir.path()).build())
            .expect("swift type generation should succeed");

        let source = fs::read_to_string(dir.path().join("App/Sources/App/App.swift"))
            .expect("should write a Swift module");

        assert_generated(
            &source,
            &[
                "public enum OperationKind: Hashable, Sendable {",
                "public var operationKind: OperationKind? {",
                "public struct EffectSink<Item>: Sendable {",
                "public protocol EffectHandler: Sendable {",
                "func render(_ operation: RenderOperation)",
                "func get(_ operation: Get) async -> GetResult",
                "func subscribe(_ operation: Subscribe, into sink: EffectSink<Message>)",
                "func legacy(_ operation: Legacy, requestId: UInt32,",
                "public struct EffectDispatcher: Sendable {",
                "public func dispatch(_ request: Request) {",
                "extension EffectHandler {",
                "public func render(_ operation: RenderOperation) {}",
                "import Observation",
                "public protocol CoreBridge: Sendable {",
                "@available(macOS 14.0, iOS 17.0, tvOS 17.0, watchOS 10.0, *)",
                "@Observable",
                "public final class Core {",
                "public private(set) var view: ViewModel",
                "public init(bridge: any CoreBridge, handler: any EffectHandler) {",
                "public func update(_ event: Event) {",
                "public func process(bytes: [UInt8]) {",
                "public func process(_ requests: [Request]) {",
            ],
        );
    }

    #[test]
    fn generates_kotlin() {
        let dir = tempfile::tempdir().expect("should create a temp dir");
        generator()
            .kotlin(&Config::builder("com.example.shared", dir.path()).build())
            .expect("kotlin type generation should succeed");

        let source = fs::read_to_string(dir.path().join("com/example/shared/Shared.kt"))
            .expect("should write a Kotlin module");

        assert_generated(
            &source,
            &[
                "enum class OperationKind {",
                "val Effect.operationKind: OperationKind?",
                "fun interface EffectSink<in T> {",
                "interface EffectHandler {",
                "suspend fun get(operation: com.example.shared.Get): com.example.shared.GetResult",
                // A sibling namespace has to be named from the root package: a
                // bare `Kit.Presence` does not resolve from inside
                // `com.example.shared`.
                "suspend fun probe(operation: com.example.shared.Kit.Probe): com.example.shared.Kit.Presence",
                "fun subscribe(operation: com.example.shared.Subscribe, sink: EffectSink<com.example.shared.Message>)",
                "fun legacy(operation: com.example.shared.Legacy, requestId: UInt, resolve: (ByteArray) -> Unit)",
                "class EffectDispatcher(",
                "suspend fun dispatch(request: Request) {",
                "import kotlinx.coroutines.flow.StateFlow",
                "interface CoreBridge {",
                "class Core(",
                "private val scope: CoroutineScope,",
                "val view: StateFlow<com.example.shared.ViewModel> = _view.asStateFlow()",
                "fun update(event: com.example.shared.Event) {",
                "fun process(bytes: ByteArray) {",
                "fun process(requests: List<Request>) {",
            ],
        );
    }

    #[test]
    fn generates_csharp() {
        let dir = tempfile::tempdir().expect("should create a temp dir");
        generator()
            .csharp(&Config::builder("Example.Shared", dir.path()).build())
            .expect("c# type generation should succeed");

        let source = fs::read_to_string(dir.path().join("Example/Shared/Shared.cs"))
            .expect("should write a C# module");

        assert_generated(
            &source,
            &[
                "public enum OperationKind",
                "public Example.Shared.OperationKind? OperationKind => this switch",
                "public interface IEffectSink<in T>",
                "public interface IEffectHandler",
                "Task<Example.Shared.GetResult> Get(Example.Shared.Get operation);",
                "void Subscribe(Example.Shared.Subscribe operation, IEffectSink<Example.Shared.Message> sink);",
                "Task<Example.Shared.Kit.Presence> Probe(Example.Shared.Kit.Probe operation);",
                // A C# `enum` has no methods, so a unit-only output is written
                // through its companion class, which the lookup finds only in
                // the emitter's spelling.
                "Example.Shared.Kit.PresenceBincode.Serialize(output, serializer);",
                "void Legacy(Example.Shared.Legacy operation, uint requestId, Action<byte[]> resolve);",
                "public sealed class EffectDispatcher",
                "public void Dispatch(Example.Shared.Request request)",
                "using System.ComponentModel;",
                "public interface ICoreBridge",
                "public sealed class Core : INotifyPropertyChanged",
                "public event PropertyChangedEventHandler? PropertyChanged;",
                "public Core(ICoreBridge bridge, IEffectHandler handler)",
                "public void Update(Example.Shared.Event @event)",
                "Example.Shared.EventBincode.Serialize(@event, serializer);",
                "public void Process(byte[] bytes)",
                "public void Process(IReadOnlyList<Example.Shared.Request> requests)",
            ],
        );
    }

    /// Runs `pnpm` and `tsc`, so this is a real compile of the generated
    /// TypeScript, not just a string check.
    #[test]
    fn generates_typescript() {
        let dir = tempfile::tempdir().expect("should create a temp dir");
        generator()
            .typescript(&Config::builder("shared_types", dir.path()).build())
            .expect("typescript type generation should succeed");

        let source = fs::read_to_string(dir.path().join("shared_types.ts"))
            .expect("should write a TypeScript module");

        assert_generated(
            &source,
            &[
                r#"import { BincodeSerializer } from "./bincode";"#,
                r#"export type OperationKind = "notify" | "request" | "stream";"#,
                "export function effectOperationKind(effect: Effect): OperationKind | undefined {",
                "export interface EffectSink<T> {",
                "export interface EffectHandler {",
                "get(operation: Get): Promise<GetResult>;",
                "subscribe(operation: Subscribe, sink: EffectSink<Message>): void;",
                "legacy(operation: Legacy, requestId: uint32, resolve: (bytes: Uint8Array) => void): void;",
                "export class EffectDispatcher {",
                "public dispatch(request: Request): void {",
                r#"import { BincodeDeserializer } from "./bincode";"#,
                "render?(operation: RenderOperation): void;",
                "export interface CoreBridge {",
                "export class Core {",
                "public update(event: Event): void {",
                "public processBytes(bytes: Uint8Array): void {",
                "public process(requests: Request[]): void {",
            ],
        );

        assert!(
            dir.path().join("shared_types.d.ts").exists(),
            "tsc should have emitted declarations, so the generated module compiles"
        );
    }

    /// The `exports` map lets Node.js resolve the package and its subpaths
    /// with the ES module resolver. A second run must not use the lockfile
    /// of an earlier run.
    #[test]
    fn the_generated_typescript_package_exports_its_modules() {
        let dir = tempfile::tempdir().expect("should create a temp dir");
        let config = Config::builder("shared_types", dir.path()).build();
        generator()
            .typescript(&config)
            .expect("typescript type generation should succeed");

        // `pnpm install` fails with this lockfile, because the lockfile has
        // no entry for the version that it names. The run succeeds only if
        // the generator removes the lockfile first.
        fs::write(
            dir.path().join("pnpm-lock.yaml"),
            "lockfileVersion: '9.0'\n\nimporters:\n  .:\n    devDependencies:\n      typescript:\n        specifier: ^1.0.0\n        version: 1.0.0\n",
        )
        .expect("should write a stale lockfile");
        generator()
            .typescript(&config)
            .expect("a second run should replace the stale lockfile");

        let manifest =
            fs::read_to_string(dir.path().join("package.json")).expect("should write package.json");
        let json: serde_json::Value =
            serde_json::from_str(&manifest).expect("package.json should be valid JSON");

        assert_eq!(json["name"], "shared_types");
        assert!(
            json["devDependencies"]["typescript"].is_string(),
            "the fields that the installer wrote should stay"
        );
        assert_eq!(json["main"], "./shared_types.js");
        assert_eq!(json["types"], "./shared_types.d.ts");
        assert_eq!(
            json["exports"],
            serde_json::json!({
                ".": { "types": "./shared_types.d.ts", "default": "./shared_types.js" },
                "./*": { "types": "./*.d.ts", "default": "./*.js" },
                "./bincode": { "types": "./bincode/index.d.ts", "default": "./bincode/index.js" },
                "./serde": { "types": "./serde/index.d.ts", "default": "./serde/index.js" },
                "./package.json": "./package.json",
            })
        );

        // Node.js and TypeScript use the first condition that matches, so
        // `types` must come before `default` in the file.
        let types = manifest
            .find(r#""types": "./*.d.ts""#)
            .expect("should export ./*");
        let default = manifest
            .find(r#""default": "./*.js""#)
            .expect("should export ./*");
        assert!(types < default, "`types` must come before `default`");

        for module in [
            "shared_types.js",
            "bincode/index.js",
            "serde/index.js",
            "package.json",
        ] {
            assert!(
                dir.path().join(module).exists(),
                "every export target should exist, but {module} does not"
            );
        }
    }

    /// `typescript()` fails unless `tsc` accepts the module, so success means
    /// it type-checks. `Probe` resolves with `Kit.Presence`, a unit-only enum
    /// in another namespace, which the dispatcher has to serialize through its
    /// free function rather than a method it lacks.
    #[test]
    fn the_generated_typescript_type_checks() {
        let dir = tempfile::tempdir().expect("should create a temp dir");
        generator()
            .typescript(&Config::builder("shared_types", dir.path()).build())
            .expect("tsc should type-check the generated module");
    }

    /// A package `tsc` rejects fails generation, rather than surfacing only in
    /// the shell's build. The installer leaves other files in the output
    /// directory alone, and the tsconfig includes every `.ts` file in it, so a
    /// file that does not type-check is enough.
    #[test]
    fn typescript_fails_when_tsc_rejects_the_package() {
        let dir = tempfile::tempdir().expect("should create a temp dir");
        fs::write(
            dir.path().join("broken.ts"),
            "export const broken: number = \"not a number\";\n",
        )
        .expect("should write a file that does not type-check");

        let error = generator()
            .typescript(&Config::builder("shared_types", dir.path()).build())
            .expect_err("tsc should reject the package");

        match error {
            TypeGenError::CommandFailed { command, status } => {
                assert_eq!(command, "pnpm exec tsc --build");
                assert!(!status.success());
            }
            other => panic!("expected `CommandFailed`, got {other:?}"),
        }
    }

    /// The same for C#, whose unit-only enums keep their bincode entry points
    /// in a companion class — for `Kit.Presence`, and for the app's `Event`,
    /// both named from the dotted `Example.Shared`.
    #[test]
    fn the_generated_csharp_compiles() {
        let dir = tempfile::tempdir().expect("should create a temp dir");
        generator()
            .csharp(&Config::builder("Example.Shared", dir.path()).build())
            .expect("c# type generation should succeed");

        build(
            "dotnet",
            &["build", "--nologo"].map(std::ffi::OsStr::new),
            dir.path(),
        );
    }

    /// The shell compiles the generated Kotlin in its own Gradle module, so
    /// type generation writes no `build.gradle.kts`. Kotlin 2.4 tries to
    /// compile a `.kts` file in a source directory as a script, so type
    /// generation also removes the file that an earlier version wrote.
    #[test]
    fn kotlin_generation_writes_no_gradle_build_script() {
        let dir = tempfile::tempdir().expect("should create a temp dir");
        let script = dir.path().join("build.gradle.kts");
        fs::write(&script, "// written by an earlier version\n")
            .expect("should write the stale build script");

        generator()
            .kotlin(&Config::builder("com.example.shared", dir.path()).build())
            .expect("kotlin type generation should succeed");

        assert!(
            !script.exists(),
            "expected no `build.gradle.kts` in the generated directory"
        );
    }

    /// The same for Kotlin. The `EffectHandler` and `Core` name the root
    /// types from `com.example.shared`, and `Kit.Presence` from its own
    /// package below it. A bare `Kit.Presence` does not resolve there, so
    /// only `kotlinc` can tell if the spelling is correct.
    #[test]
    fn the_generated_kotlin_compiles() {
        let dir = tempfile::tempdir().expect("should create a temp dir");
        generator()
            .kotlin(&Config::builder("com.example.shared", dir.path()).build())
            .expect("kotlin type generation should succeed");

        kotlinc_build(dir.path());
    }

    /// Reads the Swift module a `CodeGenerator` writes, for the tests that
    /// only care about what is and is not in it.
    fn swift_source(generator: &crux_core::type_generation::facet::CodeGenerator) -> String {
        let dir = tempfile::tempdir().expect("should create a temp dir");
        generator
            .swift(&Config::builder("App", dir.path()).build())
            .expect("swift type generation should succeed");

        fs::read_to_string(dir.path().join("App/Sources/App/App.swift"))
            .expect("should write a Swift module")
    }

    // -----------------------------------------------------------------------
    // Bridging to the BoltFFI bindings
    // -----------------------------------------------------------------------

    /// Swift is the language that needs the most from the installer: a
    /// companion file, a package dependency, a target dependency and a
    /// deployment-target floor.
    #[test]
    fn generates_swift_ffi_bridge() {
        let dir = tempfile::tempdir().expect("should create a temp dir");
        generator()
            .boltffi(BoltFfi::new().swift("Shared"))
            .swift(
                &Config::builder("App", dir.path())
                    .platform(".iOS(.v16)")
                    .platform(".macOS(.v13)")
                    .build(),
            )
            .expect("swift type generation should succeed");

        let manifest = fs::read_to_string(dir.path().join("App/Package.swift"))
            .expect("should write a package manifest");
        assert_generated(
            &manifest,
            &[
                "platforms: [.iOS(.v16), .macOS(.v13)],",
                r#".package(path: "../Shared")"#,
                r#".product(name: "Shared", package: "Shared")"#,
            ],
        );

        let bridge = fs::read_to_string(dir.path().join("App/Sources/App/FfiBridge.swift"))
            .expect("should write the bridge beside the module");
        assert_generated(
            &bridge,
            &[
                "import Foundation",
                "import Shared",
                "public struct FfiBridge: CoreBridge, @unchecked Sendable {",
                "private let ffi = Shared.CoreFfi()",
                "[UInt8](ffi.update(data: Data(event)))",
            ],
        );

        let source = fs::read_to_string(dir.path().join("App/Sources/App/App.swift"))
            .expect("should write a Swift module");
        assert_generated(
            &source,
            &["public convenience init(handler: any EffectHandler) {"],
        );
        // The FFI module is named by the bridge alone.
        assert!(!source.contains("import Shared"));
    }

    /// Without the configuration nothing changes: no bridge, no dependency, no
    /// platforms.
    #[test]
    fn swift_without_boltffi_has_no_bridge() {
        let dir = tempfile::tempdir().expect("should create a temp dir");
        generator()
            .swift(&Config::builder("App", dir.path()).build())
            .expect("swift type generation should succeed");

        assert!(!dir.path().join("App/Sources/App/FfiBridge.swift").exists());

        let manifest = fs::read_to_string(dir.path().join("App/Package.swift"))
            .expect("should write a package manifest");
        assert!(!manifest.contains("platforms:"));
        assert!(!manifest.contains("../Shared"));

        let source = fs::read_to_string(dir.path().join("App/Sources/App/App.swift"))
            .expect("should write a Swift module");
        assert!(!source.contains("FfiBridge"));
        assert!(!source.contains("convenience init"));
    }

    #[test]
    fn generates_kotlin_ffi_bridge() {
        let dir = tempfile::tempdir().expect("should create a temp dir");
        generator()
            .boltffi(BoltFfi::new().kotlin())
            .kotlin(&Config::builder("com.example.shared", dir.path()).build())
            .expect("kotlin type generation should succeed");

        let bridge = fs::read_to_string(dir.path().join("com/example/shared/FfiBridge.kt"))
            .expect("should write the bridge beside the module");
        assert_generated(
            &bridge,
            &[
                "package com.example.shared",
                "class FfiBridge(private val ffi: CoreFfi = CoreFfi()) : CoreBridge, AutoCloseable {",
                "override fun close() = ffi.close()",
            ],
        );
        // `CoreFfi` is in the package the file declares, so no import.
        assert!(!bridge.contains("import com.example.shared.CoreFfi"));

        let source = fs::read_to_string(dir.path().join("com/example/shared/Shared.kt"))
            .expect("should write a Kotlin module");
        assert_generated(
            &source,
            &[
                "constructor(handler: EffectHandler, scope: CoroutineScope) : this(FfiBridge(), handler, scope)",
            ],
        );
    }

    #[test]
    fn kotlin_ffi_bridge_in_another_package() {
        let dir = tempfile::tempdir().expect("should create a temp dir");
        generator()
            .boltffi(BoltFfi::new().kotlin_package("com.example.ffi"))
            .kotlin(&Config::builder("com.example.shared", dir.path()).build())
            .expect("kotlin type generation should succeed");

        let bridge = fs::read_to_string(dir.path().join("com/example/shared/FfiBridge.kt"))
            .expect("should write the bridge beside the module");
        assert_generated(&bridge, &["import com.example.ffi.CoreFfi"]);
    }

    #[test]
    fn generates_csharp_ffi_bridge() {
        let dir = tempfile::tempdir().expect("should create a temp dir");
        generator()
            .boltffi(BoltFfi::new().csharp())
            .csharp(&Config::builder("Example.Shared", dir.path()).build())
            .expect("c# type generation should succeed");

        let bridge = fs::read_to_string(dir.path().join("Example/Shared/FfiBridge.cs"))
            .expect("should write the bridge beside the module");
        assert_generated(
            &bridge,
            &[
                "namespace Example.Shared;",
                "public sealed class FfiBridge : ICoreBridge, IDisposable",
                "private readonly CoreFfi _ffi = new();",
                "public void Dispose() => _ffi.Dispose();",
            ],
        );

        let source = fs::read_to_string(dir.path().join("Example/Shared/Shared.cs"))
            .expect("should write a C# module");
        assert_generated(
            &source,
            &["public Core(IEffectHandler handler) : this(new FfiBridge(), handler) {}"],
        );
    }

    /// Runs `pnpm` and `tsc` against a stub `shared` package, so the bridge is
    /// really type-checked against the shape `BoltFFI` emits.
    #[test]
    fn generates_typescript_ffi_bridge() {
        let dir = tempfile::tempdir().expect("should create a temp dir");
        let pkg = dir.path().join("pkg");
        fs::create_dir_all(&pkg).expect("should create the stub package");
        fs::write(
            pkg.join("package.json"),
            r#"{ "name": "shared", "version": "0.1.0", "main": "./shared.js", "types": "./shared.d.ts" }"#,
        )
        .expect("should write the stub manifest");
        fs::write(pkg.join("shared.js"), "module.exports = {};\n")
            .expect("should write the stub module");
        fs::write(
            pkg.join("shared.d.ts"),
            "export declare class CoreFfi {\n\
             \x20 static new(): CoreFfi;\n\
             \x20 update(event: Uint8Array): Uint8Array;\n\
             \x20 resolve(id: number, output: Uint8Array): Uint8Array;\n\
             \x20 view(): Uint8Array;\n\
             }\n",
        )
        .expect("should write the stub declarations");

        let types = dir.path().join("types");
        generator()
            .boltffi(BoltFfi::new().typescript("shared", PackageLocation::Path("../pkg".into())))
            .typescript(&Config::builder("shared_types", &types).build())
            .expect("tsc should type-check the generated module");

        let manifest =
            fs::read_to_string(types.join("package.json")).expect("should write a package.json");
        assert_generated(&manifest, &[r#""shared": "file:../pkg""#]);

        let source =
            fs::read_to_string(types.join("shared_types.ts")).expect("should write a TS module");
        assert_generated(
            &source,
            &[
                r#"import * as boltffi from "shared";"#,
                "export class FfiBridge implements CoreBridge {",
                "private readonly ffi = boltffi.CoreFfi.new();",
                "public static async create(",
                "await (boltffi as unknown as { initialized: Promise<void> }).initialized;",
            ],
        );

        assert!(
            types.join("shared_types.d.ts").exists(),
            "tsc should have emitted declarations, so the generated module compiles"
        );
    }

    /// The bridge is part of the generated `Core`, so asking for one without a
    /// `Core` is an error rather than a silent no-op.
    #[test]
    fn boltffi_needs_the_core() {
        let dir = tempfile::tempdir().expect("should create a temp dir");
        let error = generator()
            .without_core()
            .boltffi(BoltFfi::new().swift("Shared"))
            .swift(&Config::builder("App", dir.path()).build())
            .expect_err("should reject the configuration");

        let crux_core::type_generation::facet::TypeGenError::Generation(message) = error else {
            panic!("expected a generation error");
        };
        assert!(
            message.contains("`boltffi`"),
            "unexpected message: {message}"
        );
        assert!(
            message.contains("without_core()"),
            "unexpected message: {message}"
        );
    }

    #[test]
    fn effect_handlers_can_be_turned_off() {
        let source = swift_source(&generator().without_effect_handlers());

        assert!(!source.contains("EffectHandler"));
        assert!(!source.contains("EffectDispatcher"));
        // `Core` is built on the dispatcher, so it goes too.
        assert!(!source.contains("CoreBridge"));
        assert!(!source.contains("public final class Core {"));
        // The kind accessor stays: a shell dispatching by hand still has to
        // know how many times to resolve.
        assert!(source.contains("public enum OperationKind: Hashable, Sendable {"));
        assert!(source.contains("public var operationKind: OperationKind? {"));
    }

    /// The kind accessor is a property *inside* the effect record, which the
    /// emitter does not declare `partial`, so a shell that turns the handler
    /// API off is the one configuration where it could be left stranded.
    #[test]
    fn csharp_compiles_without_the_effect_handler_api() {
        let dir = tempfile::tempdir().expect("should create a temp dir");
        generator()
            .without_effect_handlers()
            .csharp(&Config::builder("Example.Shared", dir.path()).build())
            .expect("c# type generation should succeed");

        let source = fs::read_to_string(dir.path().join("Example/Shared/Shared.cs"))
            .expect("should write a C# module");
        assert!(source.contains("public enum OperationKind"));
        assert!(source.contains("public Example.Shared.OperationKind? OperationKind"));
        assert!(!source.contains("IEffectHandler"));

        build(
            "dotnet",
            &["build", "--nologo"].map(std::ffi::OsStr::new),
            dir.path(),
        );
    }

    #[test]
    fn core_can_be_turned_off() {
        let source = swift_source(&generator().without_core());

        assert!(source.contains("public protocol EffectHandler: Sendable {"));
        assert!(source.contains("public struct EffectDispatcher: Sendable {"));
        assert!(!source.contains("CoreBridge"));
        assert!(!source.contains("public final class Core {"));
        // `Observation` is imported only for `Core`.
        assert!(!source.contains("import Observation"));
    }
}

/// An effect with nothing to render has no view loop to own, so it gets no
/// `Core` — but it keeps the handler API, so the shell can drive the
/// dispatcher itself.
#[cfg(feature = "facet_typegen")]
mod facet_no_render {
    use std::fs;

    use crux_core::{
        Command,
        macros::{Operation, effect},
        type_generation::facet::{BoltFfi, Config, TypeGenError, TypeRegistry},
    };
    use facet::Facet;
    use serde::{Deserialize, Serialize};

    #[derive(Facet)]
    #[repr(C)]
    pub enum Event {
        None,
    }

    #[derive(Facet)]
    pub struct ViewModel;

    #[allow(clippy::unsafe_derive_deserialize)]
    #[derive(Operation, Facet, Debug, Clone, Serialize, Deserialize)]
    #[operation(request, output = Vec<u8>)]
    pub struct Get {
        pub key: String,
    }

    #[effect(facet_typegen)]
    pub enum Effect {
        Get(Get),
    }

    #[derive(Default)]
    pub struct App;

    impl crux_core::App for App {
        type Event = Event;
        type Model = ();
        type ViewModel = ViewModel;
        type Effect = Effect;

        fn update(&self, _event: Event, _model: &mut Self::Model) -> Command<Effect, Event> {
            Command::done()
        }

        fn view(&self, _model: &Self::Model) -> Self::ViewModel {
            ViewModel
        }
    }

    #[test]
    fn no_core_is_generated_without_a_render_variant() {
        let dir = tempfile::tempdir().expect("should create a temp dir");
        TypeRegistry::new()
            .register_app::<App>()
            .expect("should register the app")
            .build()
            .expect("should build the registry")
            .swift(&Config::builder("App", dir.path()).build())
            .expect("swift type generation should succeed");

        let source = fs::read_to_string(dir.path().join("App/Sources/App/App.swift"))
            .expect("should write a Swift module");

        assert!(source.contains("public protocol EffectHandler: Sendable {"));
        assert!(!source.contains("CoreBridge"));
        assert!(!source.contains("public final class Core {"));
        // `Observation` is imported only for `Core`.
        assert!(!source.contains("import Observation"));
    }

    /// …so there is nothing for a bridge to be the bridge of.
    #[test]
    fn boltffi_needs_a_render_variant() {
        let dir = tempfile::tempdir().expect("should create a temp dir");
        let error = TypeRegistry::new()
            .register_app::<App>()
            .expect("should register the app")
            .build()
            .expect("should build the registry")
            .boltffi(BoltFfi::new().swift("Shared"))
            .swift(&Config::builder("App", dir.path()).build())
            .expect_err("should reject the configuration");

        let TypeGenError::Generation(message) = error else {
            panic!("expected a generation error");
        };
        assert!(
            message.contains("`boltffi`"),
            "unexpected message: {message}"
        );
        assert!(
            message.contains("render variant"),
            "unexpected message: {message}"
        );
    }
}

/// Types only the generated shell API names. `Probe` is at the root and
/// resolves with `kit::Presence`, and the view model is in `ui`, so the
/// app's module refers to neither namespace except through the handler API and
/// `Core` — and has to import both all the same.
#[cfg(feature = "facet_typegen")]
mod facet_plugin_referenced_types {
    use std::fs;

    use crux_core::{
        Command,
        macros::{Operation, effect},
        render::RenderOperation,
        type_generation::facet::{Config, TypeRegistry},
    };
    use facet::Facet;
    use serde::{Deserialize, Serialize};

    use super::toolchain::swift_build;

    #[derive(Facet)]
    #[repr(C)]
    pub enum Event {
        None,
    }

    #[derive(Facet)]
    #[facet(facet_generate_attrs::namespace = "ui")]
    pub struct ViewModel {
        pub count: u32,
    }

    #[derive(Facet, Debug, Clone, Serialize, Deserialize)]
    #[repr(C)]
    #[facet(facet_generate_attrs::namespace = "kit")]
    pub enum Presence {
        Present,
        Absent,
    }

    #[allow(clippy::unsafe_derive_deserialize)]
    #[derive(Operation, Facet, Debug, Clone, Serialize, Deserialize)]
    #[operation(request, output = Presence)]
    pub struct Probe;

    #[effect(facet_typegen)]
    pub enum Effect {
        Render(RenderOperation),
        Probe(Probe),
    }

    #[derive(Default)]
    pub struct App;

    impl crux_core::App for App {
        type Event = Event;
        type Model = ();
        type ViewModel = ViewModel;
        type Effect = Effect;

        fn update(&self, _event: Event, _model: &mut Self::Model) -> Command<Effect, Event> {
            Command::done()
        }

        fn view(&self, _model: &Self::Model) -> Self::ViewModel {
            ViewModel { count: 0 }
        }
    }

    fn generator() -> crux_core::type_generation::facet::CodeGenerator {
        TypeRegistry::new()
            .register_app::<App>()
            .expect("should register the app")
            .build()
            .expect("should build the registry")
    }

    fn occurrences(source: &str, fragment: &str) -> usize {
        source.matches(fragment).count()
    }

    /// `typescript()` fails unless `tsc` accepts the package, so success is a
    /// compile check.
    #[test]
    fn typescript_imports_namespaces_only_the_shell_api_names() {
        let dir = tempfile::tempdir().expect("should create a temp dir");
        generator()
            .typescript(&Config::builder("shared_types", dir.path()).build())
            .expect("tsc should type-check the generated package");

        let source = fs::read_to_string(dir.path().join("shared_types.ts"))
            .expect("should write a TypeScript module");

        assert!(source.contains("probe(operation: Probe): Promise<Kit.Presence>;"));
        assert_eq!(
            occurrences(&source, "import * as Kit from"),
            1,
            "expected `Kit` imported once:\n{source}"
        );
        assert_eq!(
            occurrences(&source, "import * as Ui from"),
            1,
            "expected `Ui` imported once:\n{source}"
        );
    }

    /// Swift has to import the namespace's module and declare the target
    /// dependency on it, or the package does not build.
    #[test]
    fn swift_imports_namespaces_only_the_shell_api_names() {
        let dir = tempfile::tempdir().expect("should create a temp dir");
        generator()
            .swift(&Config::builder("App", dir.path()).build())
            .expect("swift type generation should succeed");

        let package = dir.path().join("App");
        let source = fs::read_to_string(package.join("Sources/App/App.swift"))
            .expect("should write a Swift module");
        let manifest =
            fs::read_to_string(package.join("Package.swift")).expect("should write a manifest");

        assert!(source.contains("func probe(_ operation: Probe) async -> Kit.Presence"));
        for module in ["Kit", "Ui"] {
            assert_eq!(
                occurrences(&source, &format!("import {module}\n")),
                1,
                "expected `{module}` imported once:\n{source}"
            );
        }

        let app_target = manifest
            .split(".target(")
            .skip(1)
            .find(|target| target.contains(r#"name: "App","#))
            .expect("should declare the app's target");
        for module in ["Kit", "Ui"] {
            assert!(
                app_target.contains(&format!(r#""{module}""#)),
                "expected the app's target to depend on `{module}`:\n{manifest}"
            );
        }

        swift_build(&package);
    }
}

/// The generated handler API claims a handful of names, so a shared type
/// cannot also use them.
#[cfg(feature = "facet_typegen")]
mod facet_clash_test {
    use crux_core::{
        Command,
        macros::effect,
        render::RenderOperation,
        type_generation::facet::{TypeGenError, TypeRegistry},
    };
    use facet::Facet;

    #[derive(Facet)]
    #[repr(C)]
    pub enum Event {
        None,
    }

    #[derive(Facet)]
    pub struct ViewModel;

    #[allow(clippy::unsafe_derive_deserialize)]
    #[derive(Facet)]
    pub struct OperationKind {
        pub whoops: String,
    }

    #[allow(clippy::unsafe_derive_deserialize)]
    #[derive(Facet)]
    pub struct Core {
        pub whoops: String,
    }

    #[allow(clippy::unsafe_derive_deserialize)]
    #[derive(Facet)]
    pub struct FfiBridge {
        pub whoops: String,
    }

    /// Two modules, each with a type whose name generates the same way.
    pub mod first {
        use facet::Facet;

        #[allow(clippy::unsafe_derive_deserialize)]
        #[derive(Facet)]
        pub struct Delete {
            pub key: String,
        }
    }

    pub mod second {
        use facet::Facet;

        #[allow(clippy::unsafe_derive_deserialize)]
        #[derive(Facet)]
        pub struct Delete {
            pub value: String,
        }
    }

    /// Reaches `second::Delete` through a field rather than registering it.
    #[allow(clippy::unsafe_derive_deserialize)]
    #[derive(Facet)]
    pub struct Holder {
        pub inner: second::Delete,
    }

    #[effect(facet_typegen)]
    pub enum Effect {
        Render(RenderOperation),
    }

    #[derive(Default)]
    pub struct App;

    impl crux_core::App for App {
        type Event = Event;
        type Model = ();
        type ViewModel = ViewModel;
        type Effect = Effect;

        fn update(&self, _event: Event, _model: &mut Self::Model) -> Command<Effect, Event> {
            Command::done()
        }

        fn view(&self, _model: &Self::Model) -> Self::ViewModel {
            ViewModel
        }
    }

    #[test]
    fn a_type_cannot_be_called_operation_kind() {
        let error = TypeRegistry::new()
            .register_app::<App>()
            .expect("should register the app")
            .register_type::<OperationKind>()
            .expect("should register the clashing type")
            .build()
            .err()
            .expect("should reject the clashing type");

        let TypeGenError::Generation(message) = error else {
            panic!("expected a generation error");
        };
        assert!(
            message.contains("`OperationKind` is generated for the shell API"),
            "unexpected message: {message}"
        );
    }

    #[test]
    fn a_type_cannot_be_called_core() {
        let error = TypeRegistry::new()
            .register_app::<App>()
            .expect("should register the app")
            .register_type::<Core>()
            .expect("should register the clashing type")
            .build()
            .err()
            .expect("should reject the clashing type");

        let TypeGenError::Generation(message) = error else {
            panic!("expected a generation error");
        };
        assert!(
            message.contains("`Core` is generated for the shell API"),
            "unexpected message: {message}"
        );
    }

    /// `FfiBridge` is reserved whether or not `boltffi` is configured: whether
    /// a shared type clashes should not depend on how the shell is wired up.
    #[test]
    fn a_type_cannot_be_called_ffi_bridge() {
        let error = TypeRegistry::new()
            .register_app::<App>()
            .expect("should register the app")
            .register_type::<FfiBridge>()
            .expect("should register the clashing type")
            .build()
            .err()
            .expect("should reject the clashing type");

        let TypeGenError::Generation(message) = error else {
            panic!("expected a generation error");
        };
        assert!(
            message.contains("`FfiBridge` is generated for the shell API"),
            "unexpected message: {message}"
        );
    }

    /// Keeping one and dropping the other silently gave shells a single
    /// `Delete` and a compile error far from the cause.
    #[test]
    fn two_types_cannot_share_a_generated_name() {
        let error = TypeRegistry::new()
            .register_type::<first::Delete>()
            .expect("should register the first type")
            .register_type::<second::Delete>()
            .err()
            .expect("should reject the clashing type");

        let TypeGenError::Generation(message) = error else {
            panic!("expected a generation error");
        };
        assert!(
            message.contains(r#"two types generate as "Delete""#),
            "unexpected message: {message}"
        );
        assert!(
            message.contains("first::Delete"),
            "unexpected message: {message}"
        );
        assert!(
            message.contains("second::Delete"),
            "unexpected message: {message}"
        );
    }

    /// The clash is just as real when the second type is reached through a
    /// field of a registered type.
    #[test]
    fn a_nested_type_cannot_share_a_generated_name() {
        let error = TypeRegistry::new()
            .register_type::<first::Delete>()
            .expect("should register the first type")
            .register_type::<Holder>()
            .err()
            .expect("should reject the clashing type");

        let TypeGenError::Generation(message) = error else {
            panic!("expected a generation error");
        };
        assert!(
            message.contains(r#"two types generate as "Delete""#),
            "unexpected message: {message}"
        );
        assert!(
            message.contains("first::Delete"),
            "unexpected message: {message}"
        );
        assert!(
            message.contains("second::Delete"),
            "unexpected message: {message}"
        );
    }
}

/// A request effect whose output is `()` should generate a Swift dispatcher
/// arm without a `let output =` binding of type `Void`.
#[cfg(feature = "facet_typegen")]
mod facet_swift_unit_output {
    #![allow(clippy::unsafe_derive_deserialize)]

    use std::fs;

    use crux_core::{
        Command,
        macros::{Operation, effect},
        render::RenderOperation,
        type_generation::facet::{Config, TypeRegistry},
    };
    use facet::Facet;
    use serde::{Deserialize, Serialize};

    use super::toolchain::swift_build;

    #[derive(Facet)]
    #[repr(C)]
    pub enum Event {
        None,
    }

    #[derive(Facet)]
    pub struct ViewModel;

    #[derive(Operation, Facet, Debug, Clone, Serialize, Deserialize)]
    #[operation(request, output = ())]
    pub struct Delay;

    #[effect(facet_typegen)]
    pub enum Effect {
        Render(RenderOperation),
        Delay(Delay),
    }

    #[derive(Default)]
    pub struct App;

    impl crux_core::App for App {
        type Event = Event;
        type Model = ();
        type ViewModel = ViewModel;
        type Effect = Effect;

        fn update(&self, _event: Event, _model: &mut Self::Model) -> Command<Effect, Event> {
            Command::done()
        }

        fn view(&self, _model: &Self::Model) -> Self::ViewModel {
            ViewModel
        }
    }

    #[test]
    fn swift_unit_output_request_omits_output_binding() {
        let dir = tempfile::tempdir().expect("should create a temp dir");
        TypeRegistry::new()
            .register_app::<App>()
            .expect("should register the app")
            .build()
            .expect("should build the registry")
            .swift(&Config::builder("App", dir.path()).build())
            .expect("swift type generation should succeed");

        let package = dir.path().join("App");
        let source = fs::read_to_string(package.join("Sources/App/App.swift"))
            .expect("should write a Swift module");

        assert!(
            source.contains("func delay(_ operation: Delay) async -> Void"),
            "expected Void return in handler protocol:\n{source}"
        );
        assert!(
            source.contains("await handler.delay(operation)"),
            "expected awaited handler call:\n{source}"
        );
        assert!(
            !source.contains("let output = await handler.delay(operation)"),
            "expected no `let output` binding for Void result:\n{source}"
        );
        assert!(
            source.contains("try serializer.serialize_unit(value: ())"),
            "expected unit literal passed to serialize_unit:\n{source}"
        );

        swift_build(&package);
    }
}

/// Helpers that compile generated code with the toolchain of each shell
/// language. A test skips a toolchain that is not installed, unless
/// `CRUX_REQUIRE_SHELL_TOOLCHAINS` is set.
#[cfg(feature = "facet_typegen")]
mod toolchain {
    use std::{
        fs,
        path::{Path, PathBuf},
        process::Command,
    };

    /// Whether a toolchain that this test cannot find is a failure, not a
    /// skip. Set `CRUX_REQUIRE_SHELL_TOOLCHAINS` where every toolchain is
    /// installed, so that a skip cannot hide a source that does not compile.
    fn toolchains_required() -> bool {
        std::env::var_os("CRUX_REQUIRE_SHELL_TOOLCHAINS").is_some_and(|value| !value.is_empty())
    }

    /// Reports a toolchain that the test did not find: a failure when
    /// `CRUX_REQUIRE_SHELL_TOOLCHAINS` is set, and a printed skip if not.
    fn unavailable(missing: &str, consequence: &str) {
        assert!(
            !toolchains_required(),
            "{missing}, so {consequence}. CRUX_REQUIRE_SHELL_TOOLCHAINS is set, so a missing \
             toolchain fails rather than skips: install it, or unset the variable."
        );
        println!("skipping: {missing}, so {consequence}");
    }

    /// Runs `program` in `dir` and asserts that it succeeded. A program that
    /// is not on `PATH` goes to [`unavailable`].
    pub fn build(program: &str, args: &[&std::ffi::OsStr], dir: &Path) {
        match Command::new(program).current_dir(dir).args(args).output() {
            Ok(output) => assert!(
                output.status.success(),
                "`{program}` failed:\n{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                unavailable(
                    &format!("`{program}` is not on PATH"),
                    "the generated code is not compiled",
                );
            }
            Err(e) => panic!("could not run `{program}`: {e}"),
        }
    }

    /// Builds a generated Swift package with `swift build`. A missing `swift`
    /// goes to [`unavailable`], as in [`build`].
    pub fn swift_build(dir: &Path) {
        build("swift", &["build"].map(std::ffi::OsStr::new), dir);
    }

    /// Compiles the generated Kotlin package directories under `dir` with
    /// `kotlinc`.
    pub fn kotlinc_build(dir: &Path) {
        let Some(coroutines) = coroutines_jar() else {
            unavailable(
                "there is no kotlinx-coroutines-core jar in KOTLINX_COROUTINES_JAR or under \
                 ~/.gradle or ~/.m2, and `kotlinc` ships only the standard library",
                "the generated Kotlin is not compiled",
            );
            return;
        };

        let mut args: Vec<std::ffi::OsString> = fs::read_dir(dir)
            .expect("should read the generated package")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .map(PathBuf::into_os_string)
            .collect();
        args.sort();
        args.extend([
            "-classpath".into(),
            coroutines.into_os_string(),
            "-d".into(),
            dir.join("classes.jar").into_os_string(),
        ]);
        let args: Vec<&std::ffi::OsStr> = args.iter().map(std::ffi::OsString::as_os_str).collect();
        build("kotlinc", &args, dir);
    }

    /// Finds a `kotlinx-coroutines-core` jar, which the generated Kotlin
    /// imports. Gradle and Maven keep one under the home directory after a
    /// build that uses it. Set `KOTLINX_COROUTINES_JAR` to use a jar in a
    /// different location.
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

    /// The first jar under `dir`, no deeper than `depth`, without the
    /// `-sources` jars.
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
            let name = path.file_name()?.to_str()?;
            if path.is_dir() {
                find_jar(path, depth - 1)
            } else {
                let jar = path
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("jar"));
                (jar && !name.ends_with("-sources.jar")).then(|| path.clone())
            }
        })
    }
}
