#[cfg(feature = "typegen")]
mod shared {
    use crux_core::Command;
    use crux_core::render::RenderOperation;
    use crux_macros::effect;
    use serde::{Deserialize, Serialize};

    #[derive(Default)]
    pub struct App;

    #[derive(Serialize, Deserialize, Debug)]
    pub enum Event {
        None,
        SendUuid(uuid::Uuid),
    }

    #[derive(Serialize, Deserialize)]
    pub struct ViewModel;
    impl crux_core::App for App {
        type Event = Event;
        type Model = ();
        type ViewModel = ViewModel;
        type Effect = Effect;

        fn update(&self, _event: Event, _model: &mut Self::Model) -> Command<Effect, Event> {
            Command::done()
        }

        fn view(&self, _model: &Self::Model) -> Self::ViewModel {
            unimplemented!();
        }
    }

    #[effect(typegen)]
    pub enum Effect {
        Render(RenderOperation),
    }
}

#[cfg(feature = "typegen")]
mod test {
    use super::shared::{App, Event};
    use crux_core::type_generation::serde::TypeGen;
    use uuid::Uuid;

    // FIXME this test is quite slow
    #[test]
    fn generate_types() {
        let mut typegen = TypeGen::new();

        let sample_events = vec![Event::SendUuid(Uuid::new_v4())];
        typegen.register_type_with_samples(sample_events).unwrap();

        typegen.register_app::<App>().unwrap();

        let temp = assert_fs::TempDir::new().unwrap();
        let output_root = temp.join("crux_core_typegen_test");

        typegen
            .swift("SharedTypes", output_root.join("swift"))
            .expect("swift type gen failed");

        typegen
            .java("com.example.counter.shared_types", output_root.join("java"))
            .expect("java type gen failed");

        typegen
            .typescript("shared_types", output_root.join("typescript"))
            .expect("typescript type gen failed");
    }

    // TODO: instead of using the Render capability here, it would be better to also test against a custom
    // capability that has an output type
    #[test]
    fn test_autodiscovery() {
        let mut typegen = TypeGen::new();

        typegen
            .register_samples(vec![Event::SendUuid(Uuid::new_v4())])
            .unwrap();

        typegen
            .register_app::<App>()
            .expect("Should register types in App");

        let registry = match typegen.state {
            crux_core::type_generation::serde::State::Registering(tracer, _) => {
                tracer.registry().expect("Should get registry")
            }
            crux_core::type_generation::serde::State::Generating(_) => {
                panic!("Expected to still be in registering stage")
            }
        };

        dbg!(&registry);

        assert!(registry.contains_key("Event"));
        assert!(registry.contains_key("ViewModel"));

        assert!(registry.contains_key("Effect"));
        assert!(registry.contains_key("RenderOperation"));
    }
}

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
    #[operation(request, output = GetResult, register(StoreError))]
    pub struct Get {
        pub key: String,
    }

    #[derive(Operation, Facet, Debug, Clone, Serialize, Deserialize)]
    #[operation(notify)]
    pub struct Publish(pub Vec<u8>);

    #[derive(Operation, Facet, Debug, Clone, Serialize, Deserialize)]
    #[operation(stream, output = Message)]
    pub struct Subscribe;

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
}

#[cfg(feature = "facet_typegen")]
mod facet_test {
    use std::fs;

    use crux_core::{
        RequestKind,
        type_generation::facet::{Config, Format, TypeRegistry},
    };

    use super::facet_shared::App;

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

    #[test]
    fn registered_types_include_those_named_by_the_derive() {
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
                "fun subscribe(operation: com.example.shared.Subscribe, sink: EffectSink<Message>)",
                "fun legacy(operation: com.example.shared.Legacy, requestId: UInt, resolve: (ByteArray) -> Unit)",
                "class EffectDispatcher(",
                "suspend fun dispatch(request: Request) {",
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
            ],
        );

        assert!(
            dir.path().join("shared_types.d.ts").exists(),
            "tsc should have emitted declarations, so the generated module compiles"
        );
    }

    #[test]
    fn effect_handlers_can_be_turned_off() {
        let dir = tempfile::tempdir().expect("should create a temp dir");
        generator()
            .without_effect_handlers()
            .swift(&Config::builder("SharedTypes", dir.path()).build())
            .expect("swift type generation should succeed");

        let source = fs::read_to_string(
            dir.path()
                .join("SharedTypes/Sources/SharedTypes/SharedTypes.swift"),
        )
        .expect("should write a Swift module");

        assert!(!source.contains("RequestKind"));
        assert!(!source.contains("EffectHandler"));
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
            message.contains("`RequestKind` is generated for the effect handler API"),
            "unexpected message: {message}"
        );
    }
}
