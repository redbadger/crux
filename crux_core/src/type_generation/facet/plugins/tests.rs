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
        plugin::{CompanionFile, EmitContext, EmitterPlugin},
        swift::Swift,
        typescript::TypeScript,
    },
};

use super::{CorePlugin, EffectHandlerPlugin, OperationKindPlugin, RequestIdPlugin};
use crate::{
    OperationKind,
    capability::Operation,
    // The `render` flag is a `TypeId` comparison with the real operation, so
    // the fixture has to use the real one too.
    render::RenderOperation,
    type_generation::facet::{AppMeta, BoltFfi, EffectMeta, PackageLocation, TypeRegistry},
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
    const KIND: Option<OperationKind> = Some(OperationKind::Request);
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
    const KIND: Option<OperationKind> = Some(OperationKind::Stream);
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

fn operation_kind<L>() -> String
where
    OperationKindPlugin: EmitterPlugin<L>,
{
    emit(|w, ctx, effects, _app| {
        let plugin = OperationKindPlugin::new(&effects.to_vec().into());
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
        let plugin = CorePlugin::new(&effects.to_vec().into(), app.clone(), None);
        EmitterPlugin::<L>::after_type(&plugin, w, ctx)
    })
}

/// The bridge configuration the `_with_boltffi` fixtures use: every language
/// named, laid out the way `boltffi pack` lays them out.
fn boltffi() -> BoltFfi {
    BoltFfi::new()
        .swift("Shared")
        .kotlin()
        .typescript("shared", PackageLocation::Path("../pkg".to_string()))
        .csharp()
}

fn core_with_boltffi<L>() -> String
where
    CorePlugin: EmitterPlugin<L>,
{
    emit(|w, ctx, effects, app| {
        let plugin = CorePlugin::new(&effects.to_vec().into(), app.clone(), Some(boltffi()));
        EmitterPlugin::<L>::after_type(&plugin, w, ctx)
    })
}

/// The one companion file the plugin asks for, as the plugin writes it — the
/// module header the generator prepends is facet-generate's business, and has
/// its own tests there.
fn companion<L>(ffi: BoltFfi) -> CompanionFile
where
    CorePlugin: EmitterPlugin<L>,
{
    let (registry, effects) = fixture();
    let mut config = CodeGeneratorConfig::new("Shared".to_string());
    config.update_from(&registry);
    let app = app_meta(&registry);

    let plugin = CorePlugin::new(&effects.into(), app, Some(ffi));
    let mut files = EmitterPlugin::<L>::companion_files(&plugin, &config);

    assert_eq!(files.len(), 1, "expected exactly one companion file");
    files.remove(0)
}

// ---------------------------------------------------------------------------
// Operation kind
// ---------------------------------------------------------------------------

#[test]
fn operation_kind_swift() {
    insta::assert_snapshot!(operation_kind::<Swift>());
}

#[test]
fn operation_kind_kotlin() {
    insta::assert_snapshot!(operation_kind::<Kotlin>());
}

#[test]
fn operation_kind_typescript() {
    insta::assert_snapshot!(operation_kind::<TypeScript>());
}

#[test]
fn operation_kind_csharp() {
    insta::assert_snapshot!(operation_kind::<CSharp>());
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

// ---------------------------------------------------------------------------
// Core, bridged to the BoltFFI bindings
// ---------------------------------------------------------------------------

#[test]
fn core_swift_with_boltffi() {
    insta::assert_snapshot!(core_with_boltffi::<Swift>());
}

#[test]
fn core_kotlin_with_boltffi() {
    insta::assert_snapshot!(core_with_boltffi::<Kotlin>());
}

#[test]
fn core_typescript_with_boltffi() {
    insta::assert_snapshot!(core_with_boltffi::<TypeScript>());
}

#[test]
fn core_csharp_with_boltffi() {
    insta::assert_snapshot!(core_with_boltffi::<CSharp>());
}

#[test]
fn ffi_bridge_swift() {
    let file = companion::<Swift>(boltffi());

    assert_eq!(file.file_name, "FfiBridge.swift");
    assert_eq!(file.imports, vec!["Foundation", "Shared"]);
    insta::assert_snapshot!(file.contents);
}

#[test]
fn ffi_bridge_kotlin() {
    let file = companion::<Kotlin>(boltffi());

    assert_eq!(file.file_name, "FfiBridge.kt");
    // `CoreFfi` is generated into the package the types are generated into.
    assert!(
        file.imports.is_empty(),
        "unexpected imports: {:?}",
        file.imports
    );
    insta::assert_snapshot!(file.contents);
}

#[test]
fn ffi_bridge_csharp() {
    let file = companion::<CSharp>(boltffi());

    assert_eq!(file.file_name, "FfiBridge.cs");
    assert!(
        file.imports.is_empty(),
        "unexpected imports: {:?}",
        file.imports
    );
    insta::assert_snapshot!(file.contents);
}

/// TypeScript emits one file per module, so there is no companion — the bridge
/// is in the module, and the package is imported wholesale because the
/// `initialized` promise is not in its type declarations.
#[test]
fn typescript_has_no_companion_and_imports_the_package() {
    let (registry, effects) = fixture();
    let mut config = CodeGeneratorConfig::new("Shared".to_string());
    config.update_from(&registry);
    let app = app_meta(&registry);

    let plugin = CorePlugin::new(&effects.into(), app, Some(boltffi()));

    assert!(EmitterPlugin::<TypeScript>::companion_files(&plugin, &config).is_empty());
    assert!(
        EmitterPlugin::<TypeScript>::imports(&plugin, &config)
            .contains(&r#"import * as boltffi from "shared";"#.to_string())
    );
    assert_eq!(
        EmitterPlugin::<TypeScript>::manifest_dependencies(&plugin),
        vec![r#""shared": "file:../pkg""#.to_string()]
    );
}

/// Swift is the only language where the generated package grows an edge: a
/// package dependency and a product on the target.
#[test]
fn swift_depends_on_the_package_the_bindings_are_in() {
    let (registry, effects) = fixture();
    let app = app_meta(&registry);
    let effects: std::sync::Arc<[EffectMeta]> = effects.into();

    let plugin = CorePlugin::new(&effects, app.clone(), Some(boltffi()));
    assert_eq!(
        EmitterPlugin::<Swift>::manifest_dependencies(&plugin),
        vec![r#".package(path: "../Shared")"#.to_string()]
    );
    assert_eq!(
        EmitterPlugin::<Swift>::target_dependencies(&plugin),
        vec![r#".product(name: "Shared", package: "Shared")"#.to_string()]
    );

    // SPM names the package by the last component of the path, whatever the
    // module is called.
    let ffi = BoltFfi::new().swift_package("Shared", "SharedLib", "../generated/Shared");
    let plugin = CorePlugin::new(&effects, app, Some(ffi));
    assert_eq!(
        EmitterPlugin::<Swift>::manifest_dependencies(&plugin),
        vec![r#".package(path: "../generated/Shared")"#.to_string()]
    );
    assert_eq!(
        EmitterPlugin::<Swift>::target_dependencies(&plugin),
        vec![r#".product(name: "SharedLib", package: "Shared")"#.to_string()]
    );
}

/// Bindings generated into another package have to be imported by name.
#[test]
fn kotlin_ffi_bridge_imports_a_foreign_package() {
    let file = companion::<Kotlin>(BoltFfi::new().kotlin_package("com.example.ffi"));

    assert_eq!(file.imports, vec!["import com.example.ffi.CoreFfi"]);

    // Naming the package the types are generated into is not a foreign
    // package, so it needs no import.
    let file = companion::<Kotlin>(BoltFfi::new().kotlin_package("Shared"));
    assert!(
        file.imports.is_empty(),
        "unexpected imports: {:?}",
        file.imports
    );
}

#[test]
fn csharp_ffi_bridge_uses_a_foreign_namespace() {
    let file = companion::<CSharp>(BoltFfi::new().csharp_namespace("Example.Ffi"));

    assert_eq!(file.imports, vec!["using Example.Ffi;"]);

    let file = companion::<CSharp>(BoltFfi::new().csharp_namespace("Shared"));
    assert!(
        file.imports.is_empty(),
        "unexpected imports: {:?}",
        file.imports
    );
}

/// The class name is threaded through every language, so a Rust type that is
/// not called `CoreFfi` still bridges.
#[test]
fn boltffi_class_can_be_renamed() {
    let ffi = boltffi().class("MyCore");

    assert!(
        companion::<Swift>(ffi.clone())
            .contents
            .contains("private let ffi = Shared.MyCore()")
    );
    assert!(
        companion::<Kotlin>(ffi.clone())
            .contents
            .contains("class FfiBridge(private val ffi: MyCore = MyCore()) :")
    );
    assert!(
        companion::<CSharp>(ffi.clone())
            .contents
            .contains("private readonly MyCore _ffi = new();")
    );

    // TypeScript has no companion, so read it out of the module instead.
    let typescript = emit(|w, ctx, effects, app| {
        let plugin = CorePlugin::new(&effects.to_vec().into(), app.clone(), Some(ffi));
        EmitterPlugin::<TypeScript>::after_type(&plugin, w, ctx)
    });
    assert!(typescript.contains("private readonly ffi = boltffi.MyCore.new();"));
}

/// A language left unnamed keeps today's output: no bridge, no constructor, no
/// dependency.
#[test]
fn an_unnamed_language_gets_no_bridge() {
    let (registry, effects) = fixture();
    let mut config = CodeGeneratorConfig::new("Shared".to_string());
    config.update_from(&registry);
    let app = app_meta(&registry);

    // Swift only.
    let plugin = CorePlugin::new(&effects.into(), app, Some(BoltFfi::new().swift("Shared")));

    assert!(EmitterPlugin::<Kotlin>::companion_files(&plugin, &config).is_empty());
    assert!(EmitterPlugin::<CSharp>::companion_files(&plugin, &config).is_empty());
    assert!(EmitterPlugin::<TypeScript>::manifest_dependencies(&plugin).is_empty());
    assert!(!EmitterPlugin::<Swift>::companion_files(&plugin, &config).is_empty());
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
        let plugin = CorePlugin::new(&effects.into(), app, None);
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
    let plugin = CorePlugin::new(&effects.clone().into(), app.clone(), None);
    assert_eq!(
        EmitterPlugin::<TypeScript>::imports(&plugin, &config),
        vec![r#"import { BincodeDeserializer } from "./bincode";"#.to_string()]
    );

    // With only notifications left, nothing else imports the serializer.
    let mut notifications = effects;
    notifications[0]
        .variants
        .retain(|variant| variant.kind == Some(OperationKind::Notify));
    let plugin = CorePlugin::new(&notifications.into(), app, None);
    assert_eq!(
        EmitterPlugin::<TypeScript>::imports(&plugin, &config),
        vec![
            r#"import { BincodeDeserializer } from "./bincode";"#.to_string(),
            r#"import { BincodeSerializer } from "./bincode";"#.to_string(),
        ]
    );
}

/// `Core` is the only generated type that is `@Observable` / raises
/// `PropertyChanged`, so the frameworks those live in are imported only when it
/// is emitted.
#[test]
fn observation_is_imported_only_when_a_core_is_emitted() {
    let (registry, effects) = fixture();
    let mut config = CodeGeneratorConfig::new("Shared".to_string());
    config.update_from(&registry);
    let app = app_meta(&registry);

    let plugin = CorePlugin::new(&effects.clone().into(), app.clone(), None);
    assert_eq!(
        EmitterPlugin::<Swift>::imports(&plugin, &config),
        vec!["Observation".to_string()]
    );
    assert_eq!(
        EmitterPlugin::<CSharp>::imports(&plugin, &config),
        vec!["using System.ComponentModel;".to_string()]
    );

    let mut without_render = effects;
    without_render[0].variants.retain(|variant| !variant.render);
    let plugin = CorePlugin::new(&without_render.into(), app, None);
    assert!(EmitterPlugin::<Swift>::imports(&plugin, &config).is_empty());
    assert!(EmitterPlugin::<CSharp>::imports(&plugin, &config).is_empty());
}

/// `Core` is the only thing in the generated Kotlin that needs coroutines, so
/// the imports and the Gradle dependency come and go with it.
#[test]
fn kotlin_asks_for_coroutines_only_when_it_emits_a_core() {
    let (registry, effects) = fixture();
    let mut config = CodeGeneratorConfig::new("Shared".to_string());
    config.update_from(&registry);
    let app = app_meta(&registry);

    let plugin = CorePlugin::new(&effects.clone().into(), app.clone(), None);
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
    let plugin = CorePlugin::new(&without_render.into(), app, None);
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
    assert_eq!(stream.kind(), OperationKind::Stream);
    assert_eq!(stream.sequence(), stream.0 & SEQUENCE_MASK);

    let request = EffectId(0x0300_0001);
    assert_eq!(request.kind(), OperationKind::Request);
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
        EmitterPlugin::<Swift>::after_type(&OperationKindPlugin::new(&effects), &mut w, &ctx)
            .expect("should write nothing");
        EmitterPlugin::<Swift>::after_type(&EffectHandlerPlugin::new(&effects), &mut w, &ctx)
            .expect("should write nothing");
        EmitterPlugin::<Swift>::after_type(&RequestIdPlugin::new(&effects), &mut w, &ctx)
            .expect("should write nothing");
        EmitterPlugin::<Swift>::after_type(&CorePlugin::new(&effects, app, None), &mut w, &ctx)
            .expect("should write nothing");
    }

    assert!(buffer.is_empty(), "expected nothing, got {buffer:?}");
}
