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

use super::{EffectHandlerPlugin, RequestKindPlugin};
use crate::{
    RequestKind,
    capability::Operation,
    type_generation::facet::{EffectMeta, TypeRegistry},
};

// ---------------------------------------------------------------------------
// A fixture effect covering all four shapes a variant can have
// ---------------------------------------------------------------------------

#[derive(Facet)]
struct RenderOperation;

impl Operation for RenderOperation {
    type Output = ();
    const KIND: Option<RequestKind> = Some(RequestKind::Notify);
}

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

/// Register the fixture and hand back everything the plugins need.
fn fixture() -> (Registry, Vec<EffectMeta>) {
    let mut registry = TypeRegistry::new();
    registry
        .register_type::<EffectFfi>()
        .expect("should register the effect")
        .register_type::<HttpResult>()
        .expect("should register the request output")
        .register_type::<Message>()
        .expect("should register the stream output");
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

/// Run one plugin hook over the fixture effect and return what it wrote.
fn emit<F>(hook: F) -> String
where
    F: FnOnce(
        &mut IndentedWriter<&mut Vec<u8>>,
        &EmitContext<'_>,
        &[EffectMeta],
    ) -> std::io::Result<()>,
{
    let (registry, effects) = fixture();
    let mut config = CodeGeneratorConfig::new("Shared".to_string());
    config.update_from(&registry);

    let (name, format) = registry
        .iter()
        .find(|(name, _)| name.name == "Effect")
        .expect("the registry should contain the effect");
    let container = Container::from((name, format));
    let ctx = EmitContext::top_level(&container, &config);

    let mut buffer = Vec::new();
    {
        let mut w = IndentedWriter::new(&mut buffer, IndentConfig::Space(4));
        hook(&mut w, &ctx, &effects).expect("the plugin should write");
    }
    String::from_utf8(buffer).expect("the plugin should write valid UTF-8")
}

fn request_kind<L>() -> String
where
    RequestKindPlugin: EmitterPlugin<L>,
{
    emit(|w, ctx, effects| {
        let plugin = RequestKindPlugin::new(&effects.to_vec().into());
        EmitterPlugin::<L>::type_body(&plugin, w, ctx)?;
        EmitterPlugin::<L>::after_type(&plugin, w, ctx)
    })
}

fn handler<L>() -> String
where
    EffectHandlerPlugin: EmitterPlugin<L>,
{
    emit(|w, ctx, effects| {
        let plugin = EffectHandlerPlugin::new(&effects.to_vec().into());
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
// The plugins keep out of the way of everything else
// ---------------------------------------------------------------------------

#[test]
fn nothing_is_emitted_for_a_type_that_is_not_the_effect() {
    let (registry, effects) = fixture();
    let mut config = CodeGeneratorConfig::new("Shared".to_string());
    config.update_from(&registry);

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
    }

    assert!(buffer.is_empty(), "expected nothing, got {buffer:?}");
}
