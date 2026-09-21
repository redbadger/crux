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
                "Task<GetResult> Get(Example.Shared.Get operation);",
                "void Subscribe(Example.Shared.Subscribe operation, IEffectSink<Message> sink);",
                "void Legacy(Example.Shared.Legacy operation, uint requestId, Action<byte[]> resolve);",
                "public sealed class EffectDispatcher",
                "public void Dispatch(Example.Shared.Request request)",
                "using System.ComponentModel;",
                "public interface ICoreBridge",
                "public sealed class Core : INotifyPropertyChanged",
                "public event PropertyChangedEventHandler? PropertyChanged;",
                "public Core(ICoreBridge bridge, IEffectHandler handler)",
                "public void Update(Event @event)",
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

/// The shell handlers a capability ships: source the capability wrote, emitted
/// into the app's own generated module when the app asks for it.
#[cfg(feature = "facet_typegen")]
mod facet_shell_handler_test {
    // `#[derive(Facet)]` generates `unsafe` methods.
    #![allow(clippy::unsafe_derive_deserialize)]

    use std::{fs, path::Path, process::Command};

    use crux_core::{
        Command as CruxCommand,
        // `register_types_facet`, which a shipped handler names to register
        // the types its source uses. Anonymous, because the derive macro of
        // the same name is imported below.
        capability::Operation as _,
        macros::{Operation, effect},
        render::RenderOperation,
        type_generation::facet::{
            CodeGenerator, Config, ShellHandler, ShellSource, TypeGenError, TypeRegistry,
        },
    };
    use facet::Facet;
    use serde::{Deserialize, Serialize};

    // -----------------------------------------------------------------------
    // A small app with one request operation, so the shipped sources have a
    // real signature to implement
    // -----------------------------------------------------------------------

    #[derive(Facet)]
    #[repr(C)]
    pub enum Event {
        None,
    }

    #[derive(Facet)]
    pub struct ViewModel;

    #[derive(Operation, Facet, Debug, Clone, Serialize, Deserialize)]
    #[operation(request, output = StoreValue)]
    pub struct Get {
        pub key: String,
    }

    #[derive(Facet, Debug, Clone, Serialize, Deserialize)]
    pub struct StoreValue {
        pub value: Vec<u8>,
    }

    #[effect(facet_typegen)]
    pub enum Effect {
        Render(RenderOperation),
        Store(Get),
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

    /// A type in a namespace of its own, so that the generated package has
    /// more than one module. Registered only by the test that checks the
    /// shipped source lands in exactly one of them.
    #[derive(Facet, Debug, Clone, Serialize, Deserialize)]
    #[facet(facet_generate_attrs::namespace = "Kit")]
    pub struct Tag {
        pub label: String,
    }

    /// A type named after the protocol a registered handler declares, for the
    /// clash test. Registered only there.
    #[derive(Facet)]
    pub struct StoreHandler {
        pub whoops: String,
    }

    /// An operation the app's `Effect` never carries, standing in for the ones
    /// a shipped source implements and the app never sends. Nothing registers
    /// it but the handler that ships it.
    #[derive(Operation, Facet, Debug, Clone, Serialize, Deserialize)]
    #[operation(request, output = StoreValue)]
    pub struct Peek {
        pub key: String,
    }

    // -----------------------------------------------------------------------
    // What a capability would ship
    // -----------------------------------------------------------------------

    const SWIFT_SOURCE: &str = r"import Foundation

public protocol StoreHandler: Sendable {
    func get(_ operation: Get) async -> StoreValue
}

public struct InMemoryStoreHandler: StoreHandler {
    public init() {}

    public func get(_ operation: Get) async -> StoreValue {
        StoreValue(value: Array(operation.key.utf8))
    }
}
";

    const KOTLIN_SOURCE: &str = r"import java.util.Locale

interface StoreHandler {
    suspend fun get(operation: Get): StoreValue
}

object InMemoryStoreHandler : StoreHandler {
    override suspend fun get(operation: Get): StoreValue =
        StoreValue(operation.key.lowercase(Locale.ROOT).map { it.code.toUByte() })
}
";

    const TYPESCRIPT_SOURCE: &str = r"export interface StoreHandler {
  get(operation: Get): Promise<StoreValue>;
}

export const inMemoryStoreHandler: StoreHandler = {
  async get(operation: Get): Promise<StoreValue> {
    return new StoreValue([operation.key.length]);
  },
};
";

    const CSHARP_SOURCE: &str = r"using System.Collections.ObjectModel;
using System.Threading.Tasks;

public interface IStoreHandler
{
    Task<StoreValue> Get(Get operation);
}

public sealed class InMemoryStoreHandler : IStoreHandler
{
    public Task<StoreValue> Get(Get operation) =>
        Task.FromResult(new StoreValue { Value = new ObservableCollection<byte>() });
}
";

    static STORE: ShellHandler = ShellHandler::new("Store")
        .swift(ShellSource::stdlib(SWIFT_SOURCE))
        .kotlin(ShellSource::stdlib(KOTLIN_SOURCE))
        .typescript(ShellSource::stdlib(TYPESCRIPT_SOURCE))
        .csharp(ShellSource::stdlib(CSHARP_SOURCE));

    /// A capability whose source implements an operation the app never sends,
    /// so registering the handler has to register that operation's types too.
    /// One operation is an operation's own `register_types_facet`; a
    /// capability with several names a `fn` in its own `shell` module.
    static SHIPS_TYPES: ShellHandler = ShellHandler::new("Peeker")
        .types(Peek::register_types_facet)
        .swift(ShellSource::stdlib("public protocol PeekerHandler {}\n"));

    /// A capability that ships Swift only — the other languages implement the
    /// methods themselves, as they do today.
    static SWIFT_ONLY: ShellHandler =
        ShellHandler::new("Clock").swift(ShellSource::stdlib("public protocol ClockHandler {}\n"));

    /// A capability whose source needs a library, which is the rare case the
    /// manifest entries are for.
    static DEPENDENT: ShellHandler = ShellHandler::new("Fetcher").kotlin(
        ShellSource::stdlib("interface FetcherHandler\n")
            .dependencies(&[r#"    implementation("com.squareup.okhttp3:okhttp:4.12.0")"#]),
    );

    /// A capability whose Swift source needs a package — which in Swift means
    /// two entries: the package, and the target's edge to its product.
    static SWIFT_DEPENDENT: ShellHandler = ShellHandler::new("Uploader").swift(
        ShellSource::stdlib("public protocol UploaderHandler {}\n")
            .dependencies(&[
                r#".package(url: "https://github.com/apple/swift-nio", from: "2.0.0")"#,
            ])
            .target_dependencies(&[r#".product(name: "NIOCore", package: "swift-nio")"#]),
    );

    /// The registry the tests generate from: the app, plus whichever shipped
    /// handlers the test registers. Registering a handler is where its own
    /// types are registered, and `build` is where its names are checked, so a
    /// test that expects a rejection uses this and the rest use
    /// [`generator`].
    fn build(handlers: &[&'static ShellHandler]) -> Result<CodeGenerator, TypeGenError> {
        let mut registry = TypeRegistry::new();
        registry
            .register_app::<App>()
            .expect("should register the app");
        for handler in handlers {
            registry
                .shell_handler(handler)
                .expect("should register the shell handler");
        }
        registry.build()
    }

    fn generator(handlers: &[&'static ShellHandler]) -> CodeGenerator {
        build(handlers).expect("should build the registry")
    }

    fn message_of(error: TypeGenError) -> String {
        let TypeGenError::Generation(message) = error else {
            panic!("expected a generation error");
        };
        message
    }

    /// Why building the registry with `handlers` was rejected.
    fn rejection(handlers: &[&'static ShellHandler]) -> String {
        match build(handlers) {
            Ok(_) => panic!("should reject the handler"),
            Err(error) => message_of(error),
        }
    }

    // -----------------------------------------------------------------------
    // Emission
    // -----------------------------------------------------------------------

    #[test]
    fn swift_writes_the_source_into_a_companion_file() {
        let dir = tempfile::tempdir().expect("should create a temp dir");
        generator(&[&STORE])
            .swift(&Config::builder("App", dir.path()).build())
            .expect("swift type generation should succeed");

        let shipped = fs::read_to_string(dir.path().join("App/Sources/App/Store.swift"))
            .expect("should write the handler beside the module");

        // Verbatim, imports and all, after the header the module needs.
        assert!(
            shipped.contains(SWIFT_SOURCE),
            "expected the shipped source verbatim:\n{shipped}"
        );
        assert!(
            shipped.contains("import Serde"),
            "no module header:\n{shipped}"
        );

        // Nothing is generated that calls it — the shell writes that itself.
        let source = fs::read_to_string(dir.path().join("App/Sources/App/App.swift"))
            .expect("should write a Swift module");
        assert!(!source.contains("StoreHandler"));
    }

    #[test]
    fn kotlin_writes_the_source_into_a_companion_file() {
        let dir = tempfile::tempdir().expect("should create a temp dir");
        generator(&[&STORE])
            .kotlin(&Config::builder("com.example.shared", dir.path()).build())
            .expect("kotlin type generation should succeed");

        let shipped = fs::read_to_string(dir.path().join("com/example/shared/Store.kt"))
            .expect("should write the handler beside the module");

        assert!(
            shipped.contains(KOTLIN_SOURCE),
            "expected the shipped source verbatim:\n{shipped}"
        );
        // The source's own imports follow the package line, which is where
        // Kotlin wants them.
        assert!(
            shipped.starts_with("package com.example.shared\n"),
            "no package header:\n{shipped}"
        );
        assert!(
            shipped.find("import java.util.Locale").unwrap()
                < shipped.find("interface StoreHandler").unwrap()
        );
    }

    #[test]
    fn csharp_writes_the_source_into_a_companion_file() {
        let dir = tempfile::tempdir().expect("should create a temp dir");
        generator(&[&STORE])
            .csharp(&Config::builder("Example.Shared", dir.path()).build())
            .expect("c# type generation should succeed");

        let shipped = fs::read_to_string(dir.path().join("Example/Shared/Store.cs"))
            .expect("should write the handler beside the module");

        assert!(
            shipped.contains(CSHARP_SOURCE),
            "expected the shipped source verbatim:\n{shipped}"
        );
        // The file-scoped namespace comes from the header, and the source's
        // own `using`s follow it — which C# 10 allows.
        assert!(
            shipped.contains("namespace Example.Shared;"),
            "no namespace header:\n{shipped}"
        );
        assert!(
            shipped.find("namespace Example.Shared;").unwrap()
                < shipped.find(CSHARP_SOURCE).unwrap(),
            "the shipped `using`s should follow the file-scoped namespace:\n{shipped}"
        );
    }

    /// TypeScript modules are one file, so the source is appended after the
    /// types rather than written beside them.
    #[test]
    fn typescript_appends_the_source_after_the_types() {
        let dir = tempfile::tempdir().expect("should create a temp dir");
        generator(&[&STORE])
            .typescript(&Config::builder("shared_types", dir.path()).build())
            .expect("typescript type generation should succeed");

        let source = fs::read_to_string(dir.path().join("shared_types.ts"))
            .expect("should write a TypeScript module");

        assert!(
            source.contains(TYPESCRIPT_SOURCE),
            "expected the shipped source verbatim:\n{source}"
        );
        assert!(
            source.find("export class StoreValue").unwrap()
                < source.find("export interface StoreHandler").unwrap(),
            "the shipped source should come after the types it names"
        );

        // `typescript()` runs `tsc`, so this is a real compile of the shipped
        // source against the generated types.
        let declarations = fs::read_to_string(dir.path().join("shared_types.d.ts"))
            .expect("tsc should have emitted declarations");
        assert!(declarations.contains("inMemoryStoreHandler"));

        let output = Command::new("pnpm")
            .current_dir(dir.path())
            .args(["exec", "tsc", "--build", "--force"])
            .output()
            .expect("should run tsc");
        assert!(
            output.status.success(),
            "tsc should type-check the shipped source:\n{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    /// A capability that does not ship a language leaves that language exactly
    /// as it was.
    #[test]
    fn a_language_without_source_emits_nothing() {
        let dir = tempfile::tempdir().expect("should create a temp dir");
        let generator = generator(&[&SWIFT_ONLY]);

        generator
            .swift(&Config::builder("App", dir.path()).build())
            .expect("swift type generation should succeed");
        assert!(dir.path().join("App/Sources/App/Clock.swift").exists());

        let kotlin = tempfile::tempdir().expect("should create a temp dir");
        generator
            .kotlin(&Config::builder("com.example.shared", kotlin.path()).build())
            .expect("kotlin type generation should succeed");
        assert!(!kotlin.path().join("com/example/shared/Clock.kt").exists());

        let csharp = tempfile::tempdir().expect("should create a temp dir");
        generator
            .csharp(&Config::builder("Example.Shared", csharp.path()).build())
            .expect("c# type generation should succeed");
        assert!(!csharp.path().join("Example/Shared/Clock.cs").exists());
    }

    /// A shipped handler is a plain type, as usable from a hand-written
    /// `switch` as from the generated dispatcher.
    #[test]
    fn without_effect_handlers_still_emits_shipped_handlers() {
        let dir = tempfile::tempdir().expect("should create a temp dir");
        generator(&[&STORE])
            .without_effect_handlers()
            .swift(&Config::builder("App", dir.path()).build())
            .expect("swift type generation should succeed");

        let shipped = fs::read_to_string(dir.path().join("App/Sources/App/Store.swift"))
            .expect("should write the handler beside the module");
        assert!(shipped.contains("public protocol StoreHandler: Sendable {"));

        let source = fs::read_to_string(dir.path().join("App/Sources/App/App.swift"))
            .expect("should write a Swift module");
        assert!(!source.contains("EffectHandler"));
    }

    /// A namespace of its own is a module of its own, and the shipped source
    /// belongs to the app's module — not to every module in the package.
    #[test]
    fn the_source_lands_in_the_root_module_only() {
        let mut registry = TypeRegistry::new();
        registry
            .register_app::<App>()
            .expect("should register the app")
            .register_type::<Tag>()
            .expect("should register the namespaced type");
        registry
            .shell_handler(&STORE)
            .expect("should register the shell handler");
        let generator = registry.build().expect("should build the registry");

        let swift = tempfile::tempdir().expect("should create a temp dir");
        generator
            .swift(&Config::builder("App", swift.path()).build())
            .expect("swift type generation should succeed");
        assert!(swift.path().join("App/Sources/App/Store.swift").exists());
        assert!(!swift.path().join("App/Sources/Kit/Store.swift").exists());

        let kotlin = tempfile::tempdir().expect("should create a temp dir");
        generator
            .kotlin(&Config::builder("com.example.shared", kotlin.path()).build())
            .expect("kotlin type generation should succeed");
        assert!(kotlin.path().join("com/example/shared/Store.kt").exists());
        assert!(
            !kotlin
                .path()
                .join("com/example/shared/Kit/Store.kt")
                .exists()
        );

        let csharp = tempfile::tempdir().expect("should create a temp dir");
        generator
            .csharp(&Config::builder("Example.Shared", csharp.path()).build())
            .expect("c# type generation should succeed");
        assert!(csharp.path().join("Example/Shared/Store.cs").exists());
        assert!(!csharp.path().join("Example/Shared/Kit/Store.cs").exists());

        let typescript = tempfile::tempdir().expect("should create a temp dir");
        generator
            .typescript(&Config::builder("shared_types", typescript.path()).build())
            .expect("typescript type generation should succeed");
        let module = fs::read_to_string(typescript.path().join("shared_types.ts"))
            .expect("should write a TypeScript module");
        assert!(module.contains("export interface StoreHandler"));
        let kit = fs::read_to_string(typescript.path().join("Kit.ts"))
            .expect("should write the namespaced module");
        assert!(!kit.contains("StoreHandler"), "unexpected module:\n{kit}");
    }

    /// A shipped source implements the whole capability, so the handler
    /// registers every type it names — including the operations this app's
    /// `Effect` never carries.
    #[test]
    fn registering_a_handler_registers_the_types_its_source_names() {
        let dir = tempfile::tempdir().expect("should create a temp dir");
        generator(&[&SHIPS_TYPES])
            .swift(&Config::builder("App", dir.path()).build())
            .expect("swift type generation should succeed");

        let source = fs::read_to_string(dir.path().join("App/Sources/App/App.swift"))
            .expect("should write a Swift module");
        assert!(
            source.contains("struct Peek"),
            "the handler's own operation should be generated:\n{source}"
        );

        // …and only because the handler asked for it.
        let without = tempfile::tempdir().expect("should create a temp dir");
        generator(&[])
            .swift(&Config::builder("App", without.path()).build())
            .expect("swift type generation should succeed");
        let source = fs::read_to_string(without.path().join("App/Sources/App/App.swift"))
            .expect("should write a Swift module");
        assert!(
            !source.contains("struct Peek"),
            "unexpected type:\n{source}"
        );
    }

    // -----------------------------------------------------------------------
    // Manifest dependencies
    // -----------------------------------------------------------------------

    #[test]
    fn dependencies_reach_the_manifest_only_when_the_handler_is_registered() {
        let dir = tempfile::tempdir().expect("should create a temp dir");
        generator(&[&DEPENDENT])
            .kotlin(&Config::builder("com.example.shared", dir.path()).build())
            .expect("kotlin type generation should succeed");

        let manifest = fs::read_to_string(dir.path().join("build.gradle.kts"))
            .expect("should write a build script");
        assert!(
            manifest.contains(r#"implementation("com.squareup.okhttp3:okhttp:4.12.0")"#),
            "unexpected manifest:\n{manifest}"
        );

        let without = tempfile::tempdir().expect("should create a temp dir");
        generator(&[])
            .kotlin(&Config::builder("com.example.shared", without.path()).build())
            .expect("kotlin type generation should succeed");

        let manifest = fs::read_to_string(without.path().join("build.gradle.kts"))
            .expect("should write a build script");
        assert!(
            !manifest.contains("okhttp"),
            "unexpected manifest:\n{manifest}"
        );
    }

    /// Swift needs a second entry for a package: the generated target's edge
    /// to the product it uses, which goes in `target_dependencies`.
    #[test]
    fn swift_target_dependencies_reach_the_generated_target() {
        let dir = tempfile::tempdir().expect("should create a temp dir");
        generator(&[&SWIFT_DEPENDENT])
            .swift(&Config::builder("App", dir.path()).build())
            .expect("swift type generation should succeed");

        let manifest = fs::read_to_string(dir.path().join("App/Package.swift"))
            .expect("should write a package manifest");
        assert!(
            manifest
                .contains(r#".package(url: "https://github.com/apple/swift-nio", from: "2.0.0")"#),
            "unexpected manifest:\n{manifest}"
        );

        // The product belongs to the generated module's target, beside the
        // local targets it already depends on.
        let target = manifest
            .split(".target(")
            .skip(1)
            .find(|declaration| declaration.contains(r#"name: "App""#))
            .expect("should declare the generated target");
        assert!(
            target.contains(r#".product(name: "NIOCore", package: "swift-nio")"#),
            "unexpected target:\n{manifest}"
        );
    }

    // -----------------------------------------------------------------------
    // Registration is checked
    // -----------------------------------------------------------------------

    #[test]
    fn two_handlers_cannot_share_a_name() {
        let message = rejection(&[&STORE, &STORE]);
        assert!(
            message.contains("two shell handlers are called `Store`"),
            "unexpected message: {message}"
        );
    }

    /// The handler names a file and a protocol in the app's own module, so it
    /// cannot take a name the app's types already use.
    #[test]
    fn a_handler_cannot_take_a_registered_type_name() {
        static CLASH: ShellHandler =
            ShellHandler::new("Get").swift(ShellSource::stdlib("public protocol GetHandler {}\n"));

        let message = rejection(&[&CLASH]);
        assert!(
            message.contains("`Get` is claimed by the `Get` shell handler"),
            "unexpected message: {message}"
        );
    }

    /// …and the protocol it declares is reserved the same way `Core` is.
    #[test]
    fn a_type_cannot_be_called_store_handler() {
        let mut registry = TypeRegistry::new();
        registry
            .register_app::<App>()
            .expect("should register the app")
            .register_type::<StoreHandler>()
            .expect("should register the clashing type");
        registry
            .shell_handler(&STORE)
            .expect("should register the shell handler");
        let message = match registry.build() {
            Ok(_) => panic!("should reject the clashing type"),
            Err(error) => message_of(error),
        };
        assert!(
            message.contains("`StoreHandler` is claimed by the `Store` shell handler"),
            "unexpected message: {message}"
        );
    }

    /// A handler called `Effect` would declare an `EffectHandler`, which is the
    /// generated one's name.
    #[test]
    fn a_handler_cannot_claim_a_generated_name() {
        static CLASH: ShellHandler = ShellHandler::new("Effect")
            .swift(ShellSource::stdlib("public protocol EffectHandler {}\n"));

        let message = rejection(&[&CLASH]);
        assert!(
            message.contains("shell handler `Effect` needs the name `EffectHandler`"),
            "unexpected message: {message}"
        );
    }

    /// The companion file sits beside the module's own source file, and the
    /// module names that after the last segment of its package — so a package
    /// ending in the handler's name would have the handler overwrite the
    /// types.
    #[test]
    fn a_handler_cannot_be_named_after_the_module_it_is_written_beside() {
        let dir = tempfile::tempdir().expect("should create a temp dir");
        let generator = generator(&[&STORE]);

        let error = generator
            .csharp(&Config::builder("Crux.Store", dir.path()).build())
            .expect_err("should reject the package that would be overwritten");
        let message = message_of(error);
        assert!(
            message.contains("the `Store` shell handler is written beside the generated module"),
            "unexpected message: {message}"
        );

        let error = generator
            .kotlin(&Config::builder("com.example.store", dir.path()).build())
            .expect_err("should reject the package that would be overwritten");
        assert!(message_of(error).contains("`Store` shell handler"));

        let error = generator
            .swift(&Config::builder("Store", dir.path()).build())
            .expect_err("should reject the package that would be overwritten");
        assert!(message_of(error).contains("`Store` shell handler"));

        // TypeScript appends the source to the module rather than writing a
        // file beside it, so there is nothing to overwrite.
        generator
            .typescript(&Config::builder("store", dir.path()).build())
            .expect("typescript type generation should succeed");
    }

    // -----------------------------------------------------------------------
    // Compiling the shipped C#
    // -----------------------------------------------------------------------

    /// Whether a toolchain this test needs, and cannot find, should fail
    /// rather than skip.
    ///
    /// Set in CI and by `just ci`, where the .NET SDK is installed and a skip
    /// would mean the generated C# went uncompiled with nobody told. Unset —
    /// a Rust contributor without the SDK still gets a green run.
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

    /// Builds a generated C# package with `dotnet`, or says why it did not.
    ///
    /// The shipped sources are the one part of a capability crate `cargo test`
    /// cannot compile, so this is where a C# handler that does not build gets
    /// caught. Missing toolchains skip unless `CRUX_REQUIRE_SHELL_TOOLCHAINS`
    /// says otherwise: a Rust contributor should not need the .NET SDK.
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
                unavailable(
                    "`dotnet` is not on PATH",
                    "the generated C# is not compiled",
                );
            }
            Err(e) => panic!("could not run `dotnet build`: {e}"),
        }
    }

    #[test]
    fn csharp_shipped_source_compiles() {
        let dir = tempfile::tempdir().expect("should create a temp dir");
        generator(&[&STORE])
            .csharp(&Config::builder("Example.Shared", dir.path()).build())
            .expect("c# type generation should succeed");

        dotnet_build(dir.path());
    }

    /// The kind accessor is a property *inside* the effect record, which the
    /// emitter does not declare `partial`, so a shell that turns the handler
    /// API off is the one configuration where it could be left stranded.
    #[test]
    fn csharp_compiles_without_the_effect_handler_api() {
        let dir = tempfile::tempdir().expect("should create a temp dir");
        generator(&[&STORE])
            .without_effect_handlers()
            .csharp(&Config::builder("Example.Shared", dir.path()).build())
            .expect("c# type generation should succeed");

        let source = fs::read_to_string(dir.path().join("Example/Shared/Shared.cs"))
            .expect("should write a C# module");
        assert!(source.contains("public enum OperationKind"));
        assert!(source.contains("public Example.Shared.OperationKind? OperationKind"));
        assert!(!source.contains("IEffectHandler"));

        dotnet_build(dir.path());
    }
}
