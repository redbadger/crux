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
        RequestKind,
        type_generation::facet::{Config, Format, TypeRegistry},
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
    fn effect_variants_carry_their_declared_request_kind() {
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
                ("Render", Some(RequestKind::Notify)),
                ("Get", Some(RequestKind::Request)),
                ("Publish", Some(RequestKind::Notify)),
                ("Subscribe", Some(RequestKind::Stream)),
                ("Probe", Some(RequestKind::Request)),
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

    /// Every generated module should carry the request kinds and the handler
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
            .swift(&Config::builder("SharedTypes", dir.path()).build())
            .expect("swift type generation should succeed");

        let source = fs::read_to_string(
            dir.path()
                .join("SharedTypes/Sources/SharedTypes/SharedTypes.swift"),
        )
        .expect("should write a Swift module");

        assert_generated(
            &source,
            &[
                "public enum RequestKind: Hashable, Sendable {",
                "public var requestKind: RequestKind? {",
                "public struct EffectSink<Item>: Sendable {",
                "public protocol EffectHandler: Sendable {",
                "func render(_ operation: RenderOperation)",
                "func get(_ operation: Get) async -> GetResult",
                "func subscribe(_ operation: Subscribe, into sink: EffectSink<Message>)",
                "func legacy(_ operation: Legacy, requestId: UInt32,",
                "public struct EffectDispatcher: Sendable {",
                "public func dispatch(_ request: Request) {",
                "public enum EffectKind: UInt8, Hashable, Sendable {",
                "case render = 0",
                "case legacy = 5",
                "public struct RequestId: Hashable, Sendable {",
                "public var effectKind: EffectKind? {",
                "public var requestKind: RequestKind {",
                "public var sequence: UInt32 {",
                "extension EffectHandler {",
                "public func render(_ operation: RenderOperation) {}",
                "public protocol CoreBridge: Sendable {",
                "public final class Core {",
                "public init(bridge: any CoreBridge, handler: any EffectHandler, onView: @escaping @MainActor (ViewModel) -> Void) {",
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
                "enum class RequestKind {",
                "val Effect.requestKind: RequestKind?",
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
                "enum class EffectKind(val index: UByte) {",
                "RENDER(0u),",
                "LEGACY(5u);",
                "data class RequestId(val rawValue: UInt) {",
                "val effectKind: EffectKind?",
                "val requestKind: RequestKind",
                "val sequence: UInt",
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
                "public enum RequestKind",
                "public Example.Shared.RequestKind? RequestKind => this switch",
                "public interface IEffectSink<in T>",
                "public interface IEffectHandler",
                "Task<GetResult> Get(Example.Shared.Get operation);",
                "void Subscribe(Example.Shared.Subscribe operation, IEffectSink<Message> sink);",
                "void Legacy(Example.Shared.Legacy operation, uint requestId, Action<byte[]> resolve);",
                "public sealed class EffectDispatcher",
                "public void Dispatch(Example.Shared.Request request)",
                "public enum EffectKind : byte",
                "Render = 0,",
                "Legacy = 5,",
                "public sealed record RequestId(uint RawValue)",
                "public Example.Shared.EffectKind? EffectKind",
                "public Example.Shared.RequestKind RequestKind",
                "public uint Sequence => RawValue & 0x7fffffu;",
                "public interface ICoreBridge",
                "public sealed class Core",
                "public Core(ICoreBridge bridge, IEffectHandler handler, Action<ViewModel> onView)",
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
                r#"export type RequestKind = "notify" | "request" | "stream";"#,
                "export function effectRequestKind(effect: Effect): RequestKind | undefined {",
                "export interface EffectSink<T> {",
                "export interface EffectHandler {",
                "get(operation: Get): Promise<GetResult>;",
                "subscribe(operation: Subscribe, sink: EffectSink<Message>): void;",
                "legacy(operation: Legacy, requestId: uint32, resolve: (bytes: Uint8Array) => void): void;",
                "export class EffectDispatcher {",
                "public dispatch(request: Request): void {",
                r#"export type EffectKind = "Render" | "Get" | "Publish" | "Subscribe" | "Probe" | "Legacy";"#,
                "export interface RequestId {",
                "export function decodeRequestId(rawValue: number): RequestId {",
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
            .swift(&Config::builder("SharedTypes", dir.path()).build())
            .expect("swift type generation should succeed");

        fs::read_to_string(
            dir.path()
                .join("SharedTypes/Sources/SharedTypes/SharedTypes.swift"),
        )
        .expect("should write a Swift module")
    }

    #[test]
    fn effect_handlers_can_be_turned_off() {
        let source = swift_source(&generator().without_effect_handlers());

        assert!(!source.contains("RequestKind"));
        assert!(!source.contains("EffectHandler"));
        assert!(!source.contains("EffectKind"));
        assert!(!source.contains("RequestId"));
        // `Core` is built on the dispatcher, so it goes too.
        assert!(!source.contains("CoreBridge"));
        assert!(!source.contains("public final class Core {"));
    }

    #[test]
    fn core_can_be_turned_off() {
        let source = swift_source(&generator().without_core());

        assert!(source.contains("public protocol EffectHandler: Sendable {"));
        assert!(source.contains("public struct EffectDispatcher: Sendable {"));
        assert!(!source.contains("CoreBridge"));
        assert!(!source.contains("public final class Core {"));
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
        type_generation::facet::{Config, TypeRegistry},
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
            .swift(&Config::builder("SharedTypes", dir.path()).build())
            .expect("swift type generation should succeed");

        let source = fs::read_to_string(
            dir.path()
                .join("SharedTypes/Sources/SharedTypes/SharedTypes.swift"),
        )
        .expect("should write a Swift module");

        assert!(source.contains("public protocol EffectHandler: Sendable {"));
        assert!(!source.contains("CoreBridge"));
        assert!(!source.contains("public final class Core {"));
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
    pub struct RequestKind {
        pub whoops: String,
    }

    #[allow(clippy::unsafe_derive_deserialize)]
    #[derive(Facet)]
    pub struct Core {
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
    fn a_type_cannot_be_called_request_kind() {
        let error = TypeRegistry::new()
            .register_app::<App>()
            .expect("should register the app")
            .register_type::<RequestKind>()
            .expect("should register the clashing type")
            .build()
            .err()
            .expect("should reject the clashing type");

        let TypeGenError::Generation(message) = error else {
            panic!("expected a generation error");
        };
        assert!(
            message.contains("`RequestKind` is generated for the shell API"),
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
}
