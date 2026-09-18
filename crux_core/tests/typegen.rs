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
        type_generation::facet::{BoltFfi, Config, Format, PackageLocation, TypeRegistry},
    };

    use super::facet_shared::{App, OtherApp};

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
                "suspend fun get(operation: com.example.shared.Get): GetResult",
                // A sibling namespace has to be named from the root package: a
                // bare `Kit.Presence` does not resolve from inside
                // `com.example.shared`.
                "suspend fun probe(operation: com.example.shared.Kit.Probe): com.example.shared.Kit.Presence",
                "fun subscribe(operation: com.example.shared.Subscribe, sink: EffectSink<Message>)",
                "fun legacy(operation: com.example.shared.Legacy, requestId: UInt, resolve: (ByteArray) -> Unit)",
                "class EffectDispatcher(",
                "suspend fun dispatch(request: Request) {",
                "import kotlinx.coroutines.flow.StateFlow",
                "interface CoreBridge {",
                "class Core(",
                "private val scope: CoroutineScope,",
                "val view: StateFlow<ViewModel> = _view.asStateFlow()",
                "fun update(event: Event) {",
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
                "void Legacy(Example.Shared.Legacy operation, uint requestId, Action<byte[]> resolve);",
                "public sealed class EffectDispatcher",
                "public void Dispatch(Example.Shared.Request request)",
                "using System.ComponentModel;",
                "public interface ICoreBridge",
                "public sealed class Core : INotifyPropertyChanged",
                "public event PropertyChangedEventHandler? PropertyChanged;",
                "public Core(ICoreBridge bridge, IEffectHandler handler)",
                "public void Update(Example.Shared.Event @event)",
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
            .expect("typescript type generation should succeed");

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

    /// Compiles the generated Kotlin package directories under `dir` with
    /// `kotlinc`. It does not compile `dir` itself, because `kotlinc` would
    /// then try to compile the `build.gradle.kts` file in it.
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

    /// Type-checks a generated TypeScript package again with `tsc`, and
    /// asserts that it succeeded. `typescript()` runs `tsc` too, but it does
    /// not check the exit status.
    pub fn tsc_build(dir: &Path) {
        build(
            "pnpm",
            &["exec", "tsc", "--build", "--force"].map(std::ffi::OsStr::new),
            dir,
        );
    }
}

#[cfg(feature = "facet_typegen")]
mod facet_shell_protocol_test {
    // `#[derive(Facet)]` generates `unsafe` methods.
    #![allow(clippy::unsafe_derive_deserialize)]

    use std::fs;

    use crux_core::{
        Command as CruxCommand,
        macros::{Operation, effect},
        render::RenderOperation,
        type_generation::facet::{
            CodeGenerator, Config, ShellProtocol, TypeGenError, TypeRegistry,
        },
    };
    use facet::Facet;
    use serde::{Deserialize, Serialize};

    use super::toolchain::{build, kotlinc_build, tsc_build};

    // -----------------------------------------------------------------------
    // A capability with a protocol: two operations that name it, and an output
    // -----------------------------------------------------------------------

    #[derive(Operation, Facet, Debug, Clone, Serialize, Deserialize)]
    #[operation(
        request,
        output = StoreResult,
        protocol = crate::facet_shell_protocol_test::STORE
    )]
    pub struct StoreGet {
        pub key: String,
    }

    #[derive(Operation, Facet, Debug, Clone, Serialize, Deserialize)]
    #[operation(
        request,
        output = StoreResult,
        protocol = crate::facet_shell_protocol_test::STORE
    )]
    pub struct StoreRemove {
        pub key: String,
    }

    #[derive(Facet, Debug, Clone, Serialize, Deserialize)]
    pub struct StoreResult {
        pub value: Vec<u8>,
    }

    /// What the capability crate does in its `shell` module: register every
    /// operation, so that the protocol's types exist whichever ones the app
    /// uses.
    fn register_store(registry: &mut TypeRegistry) -> Result<&mut TypeRegistry, TypeGenError> {
        use crux_core::capability::Operation as _;

        StoreGet::register_types_facet(registry)?;
        StoreRemove::register_types_facet(registry)
    }

    const SWIFT_PROTOCOL: &str = r"import Foundation

public protocol StoreHandler: Sendable {
    func get(_ operation: StoreGet) async -> StoreResult
    func remove(_ operation: StoreRemove) async -> StoreResult
}
";

    const KOTLIN_PROTOCOL: &str = r"interface StoreHandler {
    suspend fun get(operation: StoreGet): StoreResult
    suspend fun remove(operation: StoreRemove): StoreResult
}
";

    const TYPESCRIPT_PROTOCOL: &str = r"export interface StoreHandler {
  get(operation: StoreGet): Promise<StoreResult>;
  remove(operation: StoreRemove): Promise<StoreResult>;
}
";

    const CSHARP_PROTOCOL: &str = r"using System.Threading.Tasks;

public interface IStoreHandler
{
    Task<StoreResult> Get(StoreGet operation);
    Task<StoreResult> Remove(StoreRemove operation);
}
";

    pub static STORE: ShellProtocol = ShellProtocol::new("Store")
        .rust_types(register_store)
        .swift(SWIFT_PROTOCOL, &["StoreHandler"])
        .kotlin(KOTLIN_PROTOCOL, &["StoreHandler"])
        .typescript(TYPESCRIPT_PROTOCOL, &["StoreHandler"])
        .csharp(CSHARP_PROTOCOL, &["IStoreHandler"]);

    // -----------------------------------------------------------------------
    // A second capability with a protocol, which the app does not use
    // -----------------------------------------------------------------------

    #[derive(Operation, Facet, Debug, Clone, Serialize, Deserialize)]
    #[operation(notify, protocol = crate::facet_shell_protocol_test::BEEPER)]
    pub struct Beep;

    fn register_beeper(registry: &mut TypeRegistry) -> Result<&mut TypeRegistry, TypeGenError> {
        use crux_core::capability::Operation as _;

        Beep::register_types_facet(registry)
    }

    pub static BEEPER: ShellProtocol = ShellProtocol::new("Beeper")
        .rust_types(register_beeper)
        .swift(
            "public protocol BeeperHandler { func beep(_ operation: Beep) async }\n",
            &["BeeperHandler"],
        )
        .typescript(
            "export interface BeeperHandler { beep(operation: Beep): Promise<void>; }\n",
            &["BeeperHandler"],
        );

    // -----------------------------------------------------------------------
    // Apps
    // -----------------------------------------------------------------------

    #[derive(Facet)]
    #[repr(C)]
    pub enum Event {
        None,
    }

    #[derive(Facet)]
    pub struct ViewModel;

    /// Uses one operation of the capability.
    pub mod one_op {
        use super::{CruxCommand, Event, RenderOperation, StoreGet, ViewModel, effect};

        #[effect(facet_typegen)]
        pub enum Effect {
            Render(RenderOperation),
            Get(StoreGet),
        }

        #[derive(Default)]
        pub struct App;

        impl crux_core::App for App {
            type Event = Event;
            type Model = ();
            type ViewModel = ViewModel;
            type Effect = Effect;

            fn update(&self, _event: Event, _model: &mut ()) -> CruxCommand<Effect, Event> {
                CruxCommand::done()
            }

            fn view(&self, _model: &()) -> ViewModel {
                ViewModel
            }
        }
    }

    /// Uses both operations of the capability.
    pub mod two_ops {
        use super::{
            CruxCommand, Event, RenderOperation, StoreGet, StoreRemove, ViewModel, effect,
        };

        #[effect(facet_typegen)]
        pub enum Effect {
            Render(RenderOperation),
            Get(StoreGet),
            Remove(StoreRemove),
        }

        #[derive(Default)]
        pub struct App;

        impl crux_core::App for App {
            type Event = Event;
            type Model = ();
            type ViewModel = ViewModel;
            type Effect = Effect;

            fn update(&self, _event: Event, _model: &mut ()) -> CruxCommand<Effect, Event> {
                CruxCommand::done()
            }

            fn view(&self, _model: &()) -> ViewModel {
                ViewModel
            }
        }
    }

    /// Uses no operation that has a protocol.
    pub mod no_ops {
        use super::{CruxCommand, Event, RenderOperation, ViewModel, effect};

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

            fn update(&self, _event: Event, _model: &mut ()) -> CruxCommand<Effect, Event> {
                CruxCommand::done()
            }

            fn view(&self, _model: &()) -> ViewModel {
                ViewModel
            }
        }
    }

    /// A type in a namespace of its own, so that the generated package has
    /// more than one module.
    #[derive(Facet, Debug, Clone, Serialize, Deserialize)]
    #[facet(facet_generate_attrs::namespace = "Kit")]
    pub struct Tag {
        pub label: String,
    }

    /// A type with the name that the protocol declares, for the clash test.
    #[derive(Facet)]
    pub struct StoreHandler {
        pub whoops: String,
    }

    fn one_op() -> CodeGenerator {
        let mut registry = TypeRegistry::new();
        registry
            .register_app::<one_op::App>()
            .expect("should register the app");
        registry.build().expect("should build the registry")
    }

    fn message_of(result: Result<CodeGenerator, TypeGenError>) -> String {
        match result {
            Ok(_) => panic!("should reject the registry"),
            Err(TypeGenError::Generation(message)) => message,
            Err(error) => panic!("expected a generation error, got {error:?}"),
        }
    }

    /// The message with which `build` rejects a registry that has only
    /// `protocols`.
    fn rejection(protocols: &[&'static ShellProtocol]) -> String {
        let mut registry = TypeRegistry::new();
        for protocol in protocols {
            registry
                .shell_protocol(protocol)
                .expect("should register the protocol");
        }
        message_of(registry.build())
    }

    fn has_type(generator: CodeGenerator, name: &str) -> bool {
        generator.registry().keys().any(|key| key.name == name)
    }

    // -----------------------------------------------------------------------
    // Emission
    // -----------------------------------------------------------------------

    /// An app that uses one operation of a capability gets the protocol of
    /// the capability in every language, with no registration, and the
    /// result compiles.
    #[test]
    fn an_app_that_uses_one_operation_gets_the_protocol() {
        let generator = one_op();

        let swift = tempfile::tempdir().expect("should create a temp dir");
        generator
            .swift(&Config::builder("App", swift.path()).build())
            .expect("swift type generation should succeed");
        let shipped = fs::read_to_string(swift.path().join("App/Sources/App/Store.swift"))
            .expect("should write the protocol beside the module");
        assert!(
            shipped.contains(SWIFT_PROTOCOL),
            "expected the protocol unchanged:\n{shipped}"
        );
        assert!(
            shipped.contains("import Serde"),
            "no module header:\n{shipped}"
        );
        build(
            "swift",
            &["build"].map(std::ffi::OsStr::new),
            &swift.path().join("App"),
        );

        let kotlin = tempfile::tempdir().expect("should create a temp dir");
        generator
            .kotlin(&Config::builder("com.example.shared", kotlin.path()).build())
            .expect("kotlin type generation should succeed");
        let shipped = fs::read_to_string(kotlin.path().join("com/example/shared/Store.kt"))
            .expect("should write the protocol beside the module");
        assert!(shipped.starts_with("package com.example.shared\n"));
        assert!(shipped.contains(KOTLIN_PROTOCOL));
        kotlinc_build(kotlin.path());

        let csharp = tempfile::tempdir().expect("should create a temp dir");
        generator
            .csharp(&Config::builder("Example.Shared", csharp.path()).build())
            .expect("c# type generation should succeed");
        let shipped = fs::read_to_string(csharp.path().join("Example/Shared/Store.cs"))
            .expect("should write the protocol beside the module");
        assert!(shipped.contains(CSHARP_PROTOCOL));
        build(
            "dotnet",
            &["build", "--nologo"].map(std::ffi::OsStr::new),
            csharp.path(),
        );

        let typescript = tempfile::tempdir().expect("should create a temp dir");
        generator
            .typescript(&Config::builder("shared_types", typescript.path()).build())
            .expect("typescript type generation should succeed");
        let module = fs::read_to_string(typescript.path().join("shared_types.ts"))
            .expect("should write a TypeScript module");
        assert!(module.contains(TYPESCRIPT_PROTOCOL));
        tsc_build(typescript.path());
    }

    /// The protocol names every operation of the capability, so the app gets
    /// all of their types, including the ones that it never sends.
    #[test]
    fn an_app_that_uses_one_operation_gets_every_type_of_the_capability() {
        assert!(has_type(one_op(), "StoreRemove"));
    }

    #[test]
    fn an_app_that_does_not_use_the_capability_gets_nothing() {
        let mut registry = TypeRegistry::new();
        registry
            .register_app::<no_ops::App>()
            .expect("should register the app");
        let generator = registry.build().expect("should build the registry");

        let swift = tempfile::tempdir().expect("should create a temp dir");
        generator
            .swift(&Config::builder("App", swift.path()).build())
            .expect("swift type generation should succeed");
        assert!(!swift.path().join("App/Sources/App/Store.swift").exists());
        assert!(!swift.path().join("App/Sources/App/Beeper.swift").exists());

        let typescript = tempfile::tempdir().expect("should create a temp dir");
        generator
            .typescript(&Config::builder("shared_types", typescript.path()).build())
            .expect("typescript type generation should succeed");
        let module = fs::read_to_string(typescript.path().join("shared_types.ts"))
            .expect("should write a TypeScript module");
        assert!(
            !module.contains("StoreHandler"),
            "unexpected module:\n{module}"
        );

        let mut registry = TypeRegistry::new();
        registry
            .register_app::<no_ops::App>()
            .expect("should register the app");
        assert!(!has_type(
            registry.build().expect("should build the registry"),
            "StoreGet"
        ));
    }

    /// Each operation registers the protocol, and the protocol registers each
    /// operation. The protocol is still recorded once, and emitted once.
    #[test]
    fn two_operations_of_one_capability_give_one_protocol() {
        let mut registry = TypeRegistry::new();
        registry
            .register_app::<two_ops::App>()
            .expect("should register the app")
            .shell_protocol(&STORE)
            .expect("should register the protocol again");
        let generator = registry.build().expect("should build the registry");

        let typescript = tempfile::tempdir().expect("should create a temp dir");
        generator
            .typescript(&Config::builder("shared_types", typescript.path()).build())
            .expect("typescript type generation should succeed");
        let module = fs::read_to_string(typescript.path().join("shared_types.ts"))
            .expect("should write a TypeScript module");
        assert_eq!(
            module.matches("export interface StoreHandler").count(),
            1,
            "expected the protocol once:\n{module}"
        );
        tsc_build(typescript.path());
    }

    /// The protocol goes once into the footer of the root module, after every
    /// type, and not into a namespaced module.
    #[test]
    fn typescript_writes_the_protocol_after_the_types_of_the_root_module() {
        let mut registry = TypeRegistry::new();
        registry
            .register_app::<one_op::App>()
            .expect("should register the app")
            .register_type::<Tag>()
            .expect("should register the namespaced type");
        let generator = registry.build().expect("should build the registry");

        let typescript = tempfile::tempdir().expect("should create a temp dir");
        generator
            .typescript(&Config::builder("shared_types", typescript.path()).build())
            .expect("typescript type generation should succeed");

        let module = fs::read_to_string(typescript.path().join("shared_types.ts"))
            .expect("should write a TypeScript module");
        let protocol = module
            .find(TYPESCRIPT_PROTOCOL)
            .expect("expected the protocol in the root module");
        assert_eq!(module.matches(TYPESCRIPT_PROTOCOL).count(), 1);
        for declaration in [
            "export class StoreGet",
            "export class StoreRemove",
            "export class StoreResult",
            "export class Core",
        ] {
            let at = module
                .find(declaration)
                .unwrap_or_else(|| panic!("expected `{declaration}`:\n{module}"));
            assert!(
                at < protocol,
                "the protocol should come after `{declaration}`:\n{module}"
            );
        }

        let kit = fs::read_to_string(typescript.path().join("Kit.ts"))
            .expect("should write the namespaced module");
        assert!(!kit.contains("StoreHandler"), "unexpected module:\n{kit}");

        let swift = tempfile::tempdir().expect("should create a temp dir");
        generator
            .swift(&Config::builder("App", swift.path()).build())
            .expect("swift type generation should succeed");
        assert!(swift.path().join("App/Sources/App/Store.swift").exists());
        assert!(!swift.path().join("App/Sources/Kit/Store.swift").exists());

        tsc_build(typescript.path());
    }

    // -----------------------------------------------------------------------
    // Name checks
    // -----------------------------------------------------------------------

    #[test]
    fn a_type_cannot_take_a_name_that_the_protocol_declares() {
        let mut registry = TypeRegistry::new();
        registry
            .register_app::<one_op::App>()
            .expect("should register the app")
            .register_type::<StoreHandler>()
            .expect("should register the type");

        let message = message_of(registry.build());
        assert!(
            message.contains("`StoreHandler` is declared by the `Store` shell protocol"),
            "unexpected message: {message}"
        );
    }

    /// The name of a protocol is a file stem, and `Store.swift` overwrites
    /// `store.swift` on a case-insensitive file system.
    #[test]
    fn two_protocols_cannot_share_a_name_in_different_case() {
        static LOWER: ShellProtocol = ShellProtocol::new("store").swift("// nothing\n", &[]);

        let message = rejection(&[&STORE, &LOWER]);
        assert!(
            message.contains("shell protocol `store` is registered after shell protocol `Store`"),
            "unexpected message: {message}"
        );
    }

    /// Crux writes the name as a file beside the module, in every language and
    /// on every file system, so the name must look like an identifier.
    #[test]
    fn a_protocol_name_must_be_a_bare_file_stem() {
        for name in [
            "",
            "../Store",
            "Http/Store",
            "Http\\Store",
            "Store.swift",
            "Store ",
            "Ünicode",
            "_Store",
            "1Store",
            "Sto\0re",
        ] {
            let protocol: &'static ShellProtocol = Box::leak(Box::new(
                ShellProtocol::new(name).swift("// nothing\n", &[]),
            ));
            let message = rejection(&[protocol]);
            assert!(
                message.contains(&format!(
                    "shell protocol `{name}` has a name that cannot be used as a file name"
                )),
                "unexpected message for {name:?}: {message}"
            );
        }
    }

    /// Windows does not create a file called `CON.swift` or `lpt1.kt`.
    #[test]
    fn a_protocol_cannot_take_a_windows_device_name() {
        for name in ["CON", "nul", "Aux", "COM1", "lpt9"] {
            let protocol: &'static ShellProtocol = Box::leak(Box::new(
                ShellProtocol::new(name).swift("// nothing\n", &[]),
            ));
            let message = rejection(&[protocol]);
            assert!(
                message.contains(&format!(
                    "shell protocol `{name}` has a name Windows reserves for a device"
                )),
                "unexpected message for {name:?}: {message}"
            );
        }
    }

    /// `FfiBridge.swift` is the file of the generated bridge, so a protocol of
    /// that name, in any case, would overwrite it.
    #[test]
    fn a_protocol_cannot_take_a_generated_file_name() {
        static BRIDGE: ShellProtocol = ShellProtocol::new("FfiBridge").swift("// nothing\n", &[]);
        static LOWER_BRIDGE: ShellProtocol =
            ShellProtocol::new("ffibridge").swift("// nothing\n", &[]);

        for protocol in [&BRIDGE, &LOWER_BRIDGE] {
            let message = rejection(&[protocol]);
            assert!(
                message.contains("would be written over the generated `FfiBridge` file"),
                "unexpected message: {message}"
            );
        }
    }

    /// A protocol cannot declare a name of the generated shell API, such as
    /// `EffectHandler`.
    #[test]
    fn a_protocol_cannot_declare_a_reserved_name() {
        static CLASH: ShellProtocol = ShellProtocol::new("Effects")
            .swift("public protocol EffectHandler {}\n", &["EffectHandler"]);

        let message = rejection(&[&CLASH]);
        assert!(
            message.contains(
                "shell protocol `Effects` needs the name `EffectHandler`, which is reserved for other use in the shell API"
            ),
            "unexpected message: {message}"
        );
    }

    /// The root module imports the serialization runtime, so a protocol cannot
    /// declare a name that the runtime brings in.
    #[test]
    fn a_protocol_cannot_declare_a_runtime_name() {
        static CLASH: ShellProtocol =
            ShellProtocol::new("Codec").swift("public protocol Serializer {}\n", &["Serializer"]);

        let message = rejection(&[&CLASH]);
        assert!(
            message.contains(
                "shell protocol `Codec` needs the name `Serializer`, which the generated module imports from the serialization runtime"
            ),
            "unexpected message: {message}"
        );
    }

    /// A namespace of the app is a module that the root module imports, so a
    /// protocol cannot declare the name of the namespace.
    #[test]
    fn a_protocol_cannot_declare_a_namespace_name() {
        static CLASH: ShellProtocol =
            ShellProtocol::new("Kits").swift("public struct Kit {}\n", &["Kit"]);

        let mut registry = TypeRegistry::new();
        registry
            .register_type::<Tag>()
            .expect("should register the namespaced type")
            .shell_protocol(&CLASH)
            .expect("should register the protocol");

        let message = message_of(registry.build());
        assert!(
            message.contains(
                "shell protocol `Kits` needs the name `Kit`, which is the name of a namespace"
            ),
            "unexpected message: {message}"
        );
    }

    /// Two protocols that declare the same name would declare it twice in one
    /// module.
    #[test]
    fn two_protocols_cannot_declare_the_same_name() {
        static OTHER_STORE: ShellProtocol = ShellProtocol::new("OtherStore")
            .swift("public protocol StoreHandler {}\n", &["StoreHandler"]);

        let message = rejection(&[&STORE, &OTHER_STORE]);
        assert!(
            message
                .contains("shell protocols `Store` and `OtherStore` both declare `StoreHandler`"),
            "unexpected message: {message}"
        );
    }

    #[test]
    fn a_protocol_cannot_be_named_after_the_module_it_is_written_beside() {
        let dir = tempfile::tempdir().expect("should create a temp dir");
        let error = one_op()
            .csharp(&Config::builder("Example.Store", dir.path()).build())
            .expect_err("should reject the package that would be overwritten");
        let TypeGenError::Generation(message) = error else {
            panic!("expected a generation error");
        };
        assert!(
            message.contains("`Store` shell protocol"),
            "unexpected message: {message}"
        );
    }

    /// The kind accessor is a property *inside* the effect record, which the
    /// emitter does not declare `partial`, so a shell that turns the handler
    /// API off is the one configuration where it could be left stranded.
    #[test]
    fn csharp_compiles_without_the_effect_handler_api() {
        let dir = tempfile::tempdir().expect("should create a temp dir");
        one_op()
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
}
