//! Snapshots of what the two plugins add to a generated module.
//!
//! Only the plugin hooks are exercised — the rest of the module is
//! facet-generate's and has its own tests — so the snapshots stay readable and
//! only move when we change what Crux emits.

// The fixture types exist only to be reflected, so their payloads are never
// read from Rust.
#![allow(dead_code)]

use facet::Facet;
use facet_generate::{
    Registry,
    generation::{
        CodeGeneratorConfig, Container,
        csharp::CSharp,
        indent::{IndentConfig, IndentedWriter},
        kotlin::Kotlin,
        plugin::{EmitContext, EmitterPlugin},
        swift::Swift,
        typescript::TypeScript,
    },
};

use super::{CorePlugin, EffectHandlerPlugin, RequestIdPlugin, RequestKindPlugin};
use crate::{
    RequestKind,
    capability::Operation,
    // The `render` flag is a `TypeId` comparison with the real operation, so
    // the fixture has to use the real one too.
    render::RenderOperation,
    type_generation::facet::{AppMeta, EffectMeta, TypeRegistry},
};

// ---------------------------------------------------------------------------
// A fixture effect covering all four shapes a variant can have
// ---------------------------------------------------------------------------

#[derive(Facet)]
struct HttpRequest {
    url: String,
}

impl Operation for HttpRequest {
    type Output = HttpResult;
    const KIND: Option<RequestKind> = Some(RequestKind::Request);
}

#[derive(Facet)]
#[repr(C)]
enum HttpResult {
    Ok(u16),
    Err(String),
}

#[derive(Facet)]
struct Subscribe;

impl Operation for Subscribe {
    type Output = Message;
    const KIND: Option<RequestKind> = Some(RequestKind::Stream);
}

#[derive(Facet)]
struct Message {
    body: Vec<u8>,
}

#[derive(Facet)]
struct LegacyOperation;

impl Operation for LegacyOperation {
    type Output = Message;
}

#[derive(Facet)]
#[repr(C)]
#[facet(rename = "Effect")]
enum EffectFfi {
    Render(RenderOperation),
    Http(HttpRequest),
    Subscribe(Subscribe),
    Legacy(LegacyOperation),
}

/// The generated `Core` names the app's event and view model, so the fixture
/// has both — the event as an enum, so `enum_type_names` is exercised.
#[derive(Facet)]
#[repr(C)]
enum Event {
    Increment,
    Say(String),
}

#[derive(Facet)]
struct ViewModel {
    count: u32,
}

/// Register the fixture and hand back everything the plugins need.
fn fixture() -> (Registry, Vec<EffectMeta>) {
    let mut registry = TypeRegistry::new();
    registry
        .register_type::<EffectFfi>()
        .expect("should register the effect")
        .register_type::<HttpResult>()
        .expect("should register the request output")
        .register_type::<Message>()
        .expect("should register the stream output")
        .register_type::<Event>()
        .expect("should register the event")
        .register_type::<ViewModel>()
        .expect("should register the view model");
    registry
        .register_effect::<EffectFfi>()
        .expect("should start recording the effect")
        .variant::<RenderOperation>("Render")
        .expect("Render")
        .variant::<HttpRequest>("Http")
        .expect("Http")
        .variant::<Subscribe>("Subscribe")
        .expect("Subscribe")
        .variant::<LegacyOperation>("Legacy")
        .expect("Legacy")
        .finish();

    let generator = registry.build().expect("should build the registry");
    let effects = generator.effects().to_vec();

    (generator.registry(), effects)
}

/// The registry names of the fixture's event and view model.
fn app_meta(registry: &Registry) -> AppMeta {
    let named = |name: &str| {
        registry
            .keys()
            .find(|key| key.name == name)
            .unwrap_or_else(|| panic!("the registry should contain {name}"))
            .clone()
    };

    AppMeta {
        event: named("Event"),
        view_model: named("ViewModel"),
    }
}

/// Run one plugin hook over the fixture effect and return what it wrote.
fn emit<F>(hook: F) -> String
where
    F: FnOnce(
        &mut IndentedWriter<&mut Vec<u8>>,
        &EmitContext<'_>,
        &[EffectMeta],
        &AppMeta,
    ) -> std::io::Result<()>,
{
    let (registry, effects) = fixture();
    let mut config = CodeGeneratorConfig::new("Shared".to_string());
    config.update_from(&registry);
    let app = app_meta(&registry);

    let (name, format) = registry
        .iter()
        .find(|(name, _)| name.name == "Effect")
        .expect("the registry should contain the effect");
    let container = Container::from((name, format));
    let ctx = EmitContext::top_level(&container, &config);

    let mut buffer = Vec::new();
    {
        let mut w = IndentedWriter::new(&mut buffer, IndentConfig::Space(4));
        hook(&mut w, &ctx, &effects, &app).expect("the plugin should write");
    }
    String::from_utf8(buffer).expect("the plugin should write valid UTF-8")
}

fn request_kind<L>() -> String
where
    RequestKindPlugin: EmitterPlugin<L>,
{
    emit(|w, ctx, effects, _app| {
        let plugin = RequestKindPlugin::new(&effects.to_vec().into());
        EmitterPlugin::<L>::type_body(&plugin, w, ctx)?;
        EmitterPlugin::<L>::after_type(&plugin, w, ctx)
    })
}

fn handler<L>() -> String
where
    EffectHandlerPlugin: EmitterPlugin<L>,
{
    emit(|w, ctx, effects, _app| {
        let plugin = EffectHandlerPlugin::new(&effects.to_vec().into());
        EmitterPlugin::<L>::after_type(&plugin, w, ctx)
    })
}

fn request_id<L>() -> String
where
    RequestIdPlugin: EmitterPlugin<L>,
{
    emit(|w, ctx, effects, _app| {
        let plugin = RequestIdPlugin::new(&effects.to_vec().into());
        EmitterPlugin::<L>::after_type(&plugin, w, ctx)
    })
}

fn core<L>() -> String
where
    CorePlugin: EmitterPlugin<L>,
{
    emit(|w, ctx, effects, app| {
        let plugin = CorePlugin::new(&effects.to_vec().into(), app.clone());
        EmitterPlugin::<L>::after_type(&plugin, w, ctx)
    })
}

// ---------------------------------------------------------------------------
// Request kind
// ---------------------------------------------------------------------------

#[test]
fn request_kind_swift() {
    insta::assert_snapshot!(request_kind::<Swift>());
}

#[test]
fn request_kind_kotlin() {
    insta::assert_snapshot!(request_kind::<Kotlin>());
}

#[test]
fn request_kind_typescript() {
    insta::assert_snapshot!(request_kind::<TypeScript>());
}

#[test]
fn request_kind_csharp() {
    insta::assert_snapshot!(request_kind::<CSharp>());
}

// ---------------------------------------------------------------------------
// Effect handler
// ---------------------------------------------------------------------------

#[test]
fn handler_swift() {
    insta::assert_snapshot!(handler::<Swift>());
}

#[test]
fn handler_kotlin() {
    insta::assert_snapshot!(handler::<Kotlin>());
}

#[test]
fn handler_typescript() {
    insta::assert_snapshot!(handler::<TypeScript>());
}

#[test]
fn handler_csharp() {
    insta::assert_snapshot!(handler::<CSharp>());
}

// ---------------------------------------------------------------------------
// Request id
// ---------------------------------------------------------------------------

#[test]
fn request_id_swift() {
    insta::assert_snapshot!(request_id::<Swift>());
}

#[test]
fn request_id_kotlin() {
    insta::assert_snapshot!(request_id::<Kotlin>());
}

#[test]
fn request_id_typescript() {
    insta::assert_snapshot!(request_id::<TypeScript>());
}

#[test]
fn request_id_csharp() {
    insta::assert_snapshot!(request_id::<CSharp>());
}

// ---------------------------------------------------------------------------
// Core
// ---------------------------------------------------------------------------

#[test]
fn core_swift() {
    insta::assert_snapshot!(core::<Swift>());
}

#[test]
fn core_kotlin() {
    insta::assert_snapshot!(core::<Kotlin>());
}

#[test]
fn core_typescript() {
    insta::assert_snapshot!(core::<TypeScript>());
}

#[test]
fn core_csharp() {
    insta::assert_snapshot!(core::<CSharp>());
}

/// The `TypeId` comparison in `EffectBuilder::variant` is easy to get wrong in
/// a way nothing else notices, so check the flag directly.
#[test]
fn a_render_variant_is_recorded() {
    let (_registry, effects) = fixture();
    let flags: Vec<_> = effects[0]
        .variants
        .iter()
        .map(|variant| (variant.ident.as_str(), variant.render))
        .collect();

    assert_eq!(
        flags,
        vec![
            ("Render", true),
            ("Http", false),
            ("Subscribe", false),
            ("Legacy", false),
        ]
    );
}

/// `Core` exists to own the view loop, so an effect with nothing to render
/// gets none.
#[test]
fn no_core_is_emitted_without_a_render_variant() {
    let (registry, mut effects) = fixture();
    effects[0].variants.retain(|variant| !variant.render);
    let mut config = CodeGeneratorConfig::new("Shared".to_string());
    config.update_from(&registry);
    let app = app_meta(&registry);

    let (name, format) = registry
        .iter()
        .find(|(name, _)| name.name == "Effect")
        .expect("the registry should contain the effect");
    // The registry still has four variants, so drop the same one from the
    // container the plugin sees.
    let container = Container::from((name, format));
    let ctx = EmitContext::top_level(&container, &config);

    let mut buffer = Vec::new();
    {
        let mut w = IndentedWriter::new(&mut buffer, IndentConfig::Space(4));
        let plugin = CorePlugin::new(&effects.into(), app);
        EmitterPlugin::<Swift>::after_type(&plugin, &mut w, &ctx).expect("should write nothing");
    }

    assert!(buffer.is_empty(), "expected nothing, got {buffer:?}");
}

/// TypeScript imports are written verbatim, so the serializer is asked for
/// only when the handler plugin has not already asked for it.
#[test]
fn typescript_imports_the_serializer_only_when_the_handler_does_not() {
    let (registry, effects) = fixture();
    let mut config = CodeGeneratorConfig::new("Shared".to_string());
    config.update_from(&registry);
    let app = app_meta(&registry);

    // The fixture has a request and a stream, so the handler serializes.
    let plugin = CorePlugin::new(&effects.clone().into(), app.clone());
    assert_eq!(
        EmitterPlugin::<TypeScript>::imports(&plugin, &config),
        vec![r#"import { BincodeDeserializer } from "./bincode";"#.to_string()]
    );

    // With only notifications left, nothing else imports the serializer.
    let mut notifications = effects;
    notifications[0]
        .variants
        .retain(|variant| variant.kind == Some(RequestKind::Notify));
    let plugin = CorePlugin::new(&notifications.into(), app);
    assert_eq!(
        EmitterPlugin::<TypeScript>::imports(&plugin, &config),
        vec![
            r#"import { BincodeDeserializer } from "./bincode";"#.to_string(),
            r#"import { BincodeSerializer } from "./bincode";"#.to_string(),
        ]
    );
}

/// `Core` is the only thing in the generated Kotlin that needs coroutines, so
/// the imports and the Gradle dependency come and go with it.
#[test]
fn kotlin_asks_for_coroutines_only_when_it_emits_a_core() {
    let (registry, effects) = fixture();
    let mut config = CodeGeneratorConfig::new("Shared".to_string());
    config.update_from(&registry);
    let app = app_meta(&registry);

    let plugin = CorePlugin::new(&effects.clone().into(), app.clone());
    assert!(
        EmitterPlugin::<Kotlin>::imports(&plugin, &config)
            .contains(&"import kotlinx.coroutines.flow.StateFlow".to_string())
    );
    assert_eq!(
        EmitterPlugin::<Kotlin>::manifest_dependencies(&plugin),
        vec![
            r#"    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.10.2")"#
                .to_string()
        ]
    );

    let mut without_render = effects;
    without_render[0].variants.retain(|variant| !variant.render);
    let plugin = CorePlugin::new(&without_render.into(), app);
    assert!(EmitterPlugin::<Kotlin>::imports(&plugin, &config).is_empty());
    assert!(EmitterPlugin::<Kotlin>::manifest_dependencies(&plugin).is_empty());
}

/// The decoder has to agree with the ids `EffectId` actually issues, so both
/// read their constants from the same place.
#[test]
fn the_emitted_layout_matches_the_ids_the_bridge_issues() {
    use super::request_id::{EFFECT_SHIFT, SEQUENCE_MASK, STREAM_BIT};
    use crate::bridge::EffectId;

    let stream = EffectId(0x0300_0001 | STREAM_BIT);

    assert_eq!(u32::from(stream.effect_index()), stream.0 >> EFFECT_SHIFT);
    assert_eq!(stream.kind(), RequestKind::Stream);
    assert_eq!(stream.sequence(), stream.0 & SEQUENCE_MASK);

    let request = EffectId(0x0300_0001);
    assert_eq!(request.kind(), RequestKind::Request);
    assert_eq!(request.0 & STREAM_BIT, 0);
}

// ---------------------------------------------------------------------------
// The plugins keep out of the way of everything else
// ---------------------------------------------------------------------------

#[test]
fn nothing_is_emitted_for_a_type_that_is_not_the_effect() {
    let (registry, effects) = fixture();
    let mut config = CodeGeneratorConfig::new("Shared".to_string());
    config.update_from(&registry);

    let app = app_meta(&registry);

    let (name, format) = registry
        .iter()
        .find(|(name, _)| name.name == "HttpResult")
        .expect("the registry should contain the output type");
    let container = Container::from((name, format));
    let ctx = EmitContext::top_level(&container, &config);

    let effects: std::sync::Arc<[EffectMeta]> = effects.into();
    let mut buffer = Vec::new();
    {
        let mut w = IndentedWriter::new(&mut buffer, IndentConfig::Space(4));
        EmitterPlugin::<Swift>::after_type(&RequestKindPlugin::new(&effects), &mut w, &ctx)
            .expect("should write nothing");
        EmitterPlugin::<Swift>::after_type(&EffectHandlerPlugin::new(&effects), &mut w, &ctx)
            .expect("should write nothing");
        EmitterPlugin::<Swift>::after_type(&RequestIdPlugin::new(&effects), &mut w, &ctx)
            .expect("should write nothing");
        EmitterPlugin::<Swift>::after_type(&CorePlugin::new(&effects, app), &mut w, &ctx)
            .expect("should write nothing");
    }

    assert!(buffer.is_empty(), "expected nothing, got {buffer:?}");
}
