use darling::{ast, util, FromDeriveInput, FromField, ToTokens};
use proc_macro2::{Literal, TokenStream};
use proc_macro_error::{abort_call_site, OptionExt};
use quote::{format_ident, quote};
use std::collections::BTreeMap;
use syn::{DeriveInput, GenericArgument, Ident, PathArguments, Type};

#[derive(FromDeriveInput, Debug)]
#[darling(attributes(effect), supports(struct_named))]
struct EffectStructReceiver {
    ident: Ident,
    name: Option<Ident>,
    data: ast::Data<util::Ignored, EffectFieldReceiver>,
}

#[derive(FromField, Debug)]
#[darling(attributes(effect))]
pub struct EffectFieldReceiver {
    ident: Option<Ident>,
    ty: Type,
    #[darling(default)]
    skip: bool,
}

struct Field {
    capability: Type,
    variant: Ident,
    skip: bool,
}

impl From<&EffectFieldReceiver> for Field {
    fn from(f: &EffectFieldReceiver) -> Self {
        let (capability, variant) = split_on_generic(&f.ty);
        Field {
            capability,
            variant,
            skip: f.skip,
        }
    }
}

impl ToTokens for EffectStructReceiver {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ident = &self.ident;

        let (effect_name, ffi_effect_name, ffi_effect_rename) = match self.name {
            Some(ref name) => {
                let ffi_ef_name = format_ident!("{}Ffi", name);
                let ffi_ef_rename = Literal::string(&name.to_string());

                (quote!(#name), quote!(#ffi_ef_name), quote!(#ffi_ef_rename))
            }
            None => (quote!(Effect), quote!(EffectFfi), quote!("Effect")),
        };

        let fields = self
            .data
            .as_ref()
            .take_struct()
            .expect_or_abort("should be a struct")
            .fields;

        let fields: BTreeMap<Ident, Field> = fields
            .into_iter()
            .map(|f| (f.ident.clone().unwrap(), f.into()))
            .collect();

        let mut variants = Vec::new();
        let mut with_context_fields = Vec::new();
        let mut ffi_variants = Vec::new();
        let mut match_arms = Vec::new();
        let mut filters = Vec::new();

        for (
            field_name,
            Field {
                capability,
                variant,
                skip,
            },
        ) in fields.iter()
        {
            if *skip {
                let msg = format!("Requesting effects from capability \"{variant}\" is impossible because it was skipped",);
                with_context_fields.push(quote! {
                    #field_name: #capability::new(context.specialize(|_| unreachable!(#msg)))
                });
            } else {
                with_context_fields.push(quote! {
                    #field_name: #capability::new(context.specialize(#effect_name::#variant))
                });

                variants.push(quote! {
                    #variant(::crux_core::Request<<#capability as ::crux_core::capability::Capability>::Operation>)
                });

                ffi_variants.push(quote! { #variant(<#capability as ::crux_core::capability::Capability>::Operation) });

                match_arms.push(quote! { #effect_name::#variant(request) => request.serialize(#ffi_effect_name::#variant) });

                let filter_fn = format_ident!("is_{}", field_name);
                let map_fn = format_ident!("into_{}", field_name);
                filters.push(quote! {
                    impl #effect_name {
                        pub fn #filter_fn(&self) -> bool {
                            if let #effect_name::#variant(_) = self {
                                true
                            } else {
                                false
                            }
                        }
                        pub fn #map_fn(self) -> Option<crux_core::Request<<#capability as ::crux_core::capability::Capability>::Operation>> {
                            if let #effect_name::#variant(request) = self {
                                Some(request)
                            } else {
                                None
                            }
                        }
                    }
                });
            }
        }

        tokens.extend(quote! {
            #[derive(Debug)]
            pub enum #effect_name {
                #(#variants ,)*
            }

            #[derive(::serde::Serialize, ::serde::Deserialize)]
            #[serde(rename = #ffi_effect_rename)]
            pub enum #ffi_effect_name {
                #(#ffi_variants ,)*
            }

            impl ::crux_core::Effect for #effect_name {
                type Ffi = #ffi_effect_name;

                fn serialize(self) -> (Self::Ffi, ::crux_core::bridge::ResolveSerialized) {
                    match self {
                        #(#match_arms ,)*
                    }
                }
            }

            impl ::crux_core::WithContext<#effect_name> for #ident {
                fn new_with_context(context: ::crux_core::capability::ProtoContext<#effect_name>) -> #ident {
                    #ident {
                        #(#with_context_fields ,)*
                    }
                }
            }

            #(#filters)*
        })
    }
}

pub(crate) fn effect_impl(input: &DeriveInput) -> TokenStream {
    let input = match EffectStructReceiver::from_derive_input(input) {
        Ok(v) => v,
        Err(e) => {
            return e.write_errors();
        }
    };

    quote!(#input)
}

fn split_on_generic(ty: &Type) -> (Type, Ident) {
    let ty = ty.clone();
    match ty {
        Type::Path(mut path) if path.qself.is_none() => {
            let last = path.path.segments.last_mut().expect("type has no segments");
            let type_name = last.ident.clone();
            Some((Type::Path(path), type_name))
        }
        _ => None,
    }
    .expect_or_abort("bad path")
}

#[cfg(test)]
mod tests {
    use darling::{FromDeriveInput, FromMeta, ToTokens};
    use quote::quote;
    use syn::{parse_str, Type};

    use crate::effect::EffectStructReceiver;

    use super::split_on_generic;

    #[test]
    fn defaults() {
        let input = r#"
            #[derive(Effect)]
            pub struct Capabilities {
                pub render: Render<Event>,
            }
        "#;
        let input = parse_str(input).unwrap();
        let input = EffectStructReceiver::from_derive_input(&input).unwrap();

        let actual = quote!(#input);

        insta::assert_snapshot!(pretty_print(&actual), @r###"
        #[derive(Debug)]
        pub enum Effect {
            Render(
                ::crux_core::Request<
                    <Render<Event> as ::crux_core::capability::Capability<Event>>::Operation,
                >,
            ),
        }
        #[derive(::serde::Serialize, ::serde::Deserialize)]
        #[serde(rename = "Effect")]
        pub enum EffectFfi {
            Render(<Render<Event> as ::crux_core::capability::Capability<Event>>::Operation),
        }
        impl ::crux_core::Effect for Effect {
            type Ffi = EffectFfi;
            fn serialize(self) -> (Self::Ffi, ::crux_core::bridge::ResolveSerialized) {
                match self {
                    Effect::Render(request) => request.serialize(EffectFfi::Render),
                }
            }
        }
        impl ::crux_core::WithContext<Event, Effect> for Capabilities {
            fn new_with_context(
                context: ::crux_core::capability::ProtoContext<Effect, Event>,
            ) -> Capabilities {
                Capabilities {
                    render: Render::new(context.specialize(Effect::Render)),
                }
            }
        }
        impl Effect {
            pub fn is_render(&self) -> bool {
                if let Effect::Render(_) = self { true } else { false }
            }
            pub fn into_render(
                self,
            ) -> Option<
                crux_core::Request<
                    <Render<Event> as ::crux_core::capability::Capability<Event>>::Operation,
                >,
            > {
                if let Effect::Render(request) = self { Some(request) } else { None }
            }
        }
        "###);
    }

    #[test]
    fn effect_skip() {
        let input = r#"
            #[derive(Effect)]
            pub struct Capabilities {
                pub render: Render<Event>,
                #[effect(skip)]
                pub compose: Compose<Event>,
            }
        "#;
        let input = parse_str(input).unwrap();
        let input = EffectStructReceiver::from_derive_input(&input).unwrap();

        let actual = quote!(#input);

        insta::assert_snapshot!(pretty_print(&actual), @r###"
        #[derive(Debug)]
        pub enum Effect {
            Render(
                ::crux_core::Request<
                    <Render<Event> as ::crux_core::capability::Capability<Event>>::Operation,
                >,
            ),
        }
        #[derive(::serde::Serialize, ::serde::Deserialize)]
        #[serde(rename = "Effect")]
        pub enum EffectFfi {
            Render(<Render<Event> as ::crux_core::capability::Capability<Event>>::Operation),
        }
        impl ::crux_core::Effect for Effect {
            type Ffi = EffectFfi;
            fn serialize(self) -> (Self::Ffi, ::crux_core::bridge::ResolveSerialized) {
                match self {
                    Effect::Render(request) => request.serialize(EffectFfi::Render),
                }
            }
        }
        impl ::crux_core::WithContext<Event, Effect> for Capabilities {
            fn new_with_context(
                context: ::crux_core::capability::ProtoContext<Effect, Event>,
            ) -> Capabilities {
                Capabilities {
                    compose: Compose::new(
                        context
                            .specialize(|_| {
                                unreachable!(
                                    "Requesting effects from capability \"Compose\" is impossible because it was skipped"
                                )
                            }),
                    ),
                    render: Render::new(context.specialize(Effect::Render)),
                }
            }
        }
        impl Effect {
            pub fn is_render(&self) -> bool {
                if let Effect::Render(_) = self { true } else { false }
            }
            pub fn into_render(
                self,
            ) -> Option<
                crux_core::Request<
                    <Render<Event> as ::crux_core::capability::Capability<Event>>::Operation,
                >,
            > {
                if let Effect::Render(request) = self { Some(request) } else { None }
            }
        }
        "###);
    }

    #[test]
    fn full() {
        let input = r#"
            #[derive(Effect)]
            #[effect(name = "MyEffect")]
            pub struct MyCapabilities {
                pub http: crux_http::Http<MyEvent>,
                pub key_value: KeyValue<MyEvent>,
                pub platform: Platform<MyEvent>,
                pub render: Render<MyEvent>,
                pub time: Time<MyEvent>,
            }
        "#;
        let input = parse_str(input).unwrap();
        let input = EffectStructReceiver::from_derive_input(&input).unwrap();

        let actual = quote!(#input);

        insta::assert_snapshot!(pretty_print(&actual), @r###"
        #[derive(Debug)]
        pub enum MyEffect {
            Http(
                ::crux_core::Request<
                    <crux_http::Http<
                        MyEvent,
                    > as ::crux_core::capability::Capability<MyEvent>>::Operation,
                >,
            ),
            KeyValue(
                ::crux_core::Request<
                    <KeyValue<
                        MyEvent,
                    > as ::crux_core::capability::Capability<MyEvent>>::Operation,
                >,
            ),
            Platform(
                ::crux_core::Request<
                    <Platform<
                        MyEvent,
                    > as ::crux_core::capability::Capability<MyEvent>>::Operation,
                >,
            ),
            Render(
                ::crux_core::Request<
                    <Render<MyEvent> as ::crux_core::capability::Capability<MyEvent>>::Operation,
                >,
            ),
            Time(
                ::crux_core::Request<
                    <Time<MyEvent> as ::crux_core::capability::Capability<MyEvent>>::Operation,
                >,
            ),
        }
        #[derive(::serde::Serialize, ::serde::Deserialize)]
        #[serde(rename = "MyEffect")]
        pub enum MyEffectFfi {
            Http(
                <crux_http::Http<
                    MyEvent,
                > as ::crux_core::capability::Capability<MyEvent>>::Operation,
            ),
            KeyValue(
                <KeyValue<MyEvent> as ::crux_core::capability::Capability<MyEvent>>::Operation,
            ),
            Platform(
                <Platform<MyEvent> as ::crux_core::capability::Capability<MyEvent>>::Operation,
            ),
            Render(<Render<MyEvent> as ::crux_core::capability::Capability<MyEvent>>::Operation),
            Time(<Time<MyEvent> as ::crux_core::capability::Capability<MyEvent>>::Operation),
        }
        impl ::crux_core::Effect for MyEffect {
            type Ffi = MyEffectFfi;
            fn serialize(self) -> (Self::Ffi, ::crux_core::bridge::ResolveSerialized) {
                match self {
                    MyEffect::Http(request) => request.serialize(MyEffectFfi::Http),
                    MyEffect::KeyValue(request) => request.serialize(MyEffectFfi::KeyValue),
                    MyEffect::Platform(request) => request.serialize(MyEffectFfi::Platform),
                    MyEffect::Render(request) => request.serialize(MyEffectFfi::Render),
                    MyEffect::Time(request) => request.serialize(MyEffectFfi::Time),
                }
            }
        }
        impl ::crux_core::WithContext<MyEvent, MyEffect> for MyCapabilities {
            fn new_with_context(
                context: ::crux_core::capability::ProtoContext<MyEffect, MyEvent>,
            ) -> MyCapabilities {
                MyCapabilities {
                    http: crux_http::Http::new(context.specialize(MyEffect::Http)),
                    key_value: KeyValue::new(context.specialize(MyEffect::KeyValue)),
                    platform: Platform::new(context.specialize(MyEffect::Platform)),
                    render: Render::new(context.specialize(MyEffect::Render)),
                    time: Time::new(context.specialize(MyEffect::Time)),
                }
            }
        }
        impl MyEffect {
            pub fn is_http(&self) -> bool {
                if let MyEffect::Http(_) = self { true } else { false }
            }
            pub fn into_http(
                self,
            ) -> Option<
                crux_core::Request<
                    <crux_http::Http<
                        MyEvent,
                    > as ::crux_core::capability::Capability<MyEvent>>::Operation,
                >,
            > {
                if let MyEffect::Http(request) = self { Some(request) } else { None }
            }
        }
        impl MyEffect {
            pub fn is_key_value(&self) -> bool {
                if let MyEffect::KeyValue(_) = self { true } else { false }
            }
            pub fn into_key_value(
                self,
            ) -> Option<
                crux_core::Request<
                    <KeyValue<
                        MyEvent,
                    > as ::crux_core::capability::Capability<MyEvent>>::Operation,
                >,
            > {
                if let MyEffect::KeyValue(request) = self { Some(request) } else { None }
            }
        }
        impl MyEffect {
            pub fn is_platform(&self) -> bool {
                if let MyEffect::Platform(_) = self { true } else { false }
            }
            pub fn into_platform(
                self,
            ) -> Option<
                crux_core::Request<
                    <Platform<
                        MyEvent,
                    > as ::crux_core::capability::Capability<MyEvent>>::Operation,
                >,
            > {
                if let MyEffect::Platform(request) = self { Some(request) } else { None }
            }
        }
        impl MyEffect {
            pub fn is_render(&self) -> bool {
                if let MyEffect::Render(_) = self { true } else { false }
            }
            pub fn into_render(
                self,
            ) -> Option<
                crux_core::Request<
                    <Render<MyEvent> as ::crux_core::capability::Capability<MyEvent>>::Operation,
                >,
            > {
                if let MyEffect::Render(request) = self { Some(request) } else { None }
            }
        }
        impl MyEffect {
            pub fn is_time(&self) -> bool {
                if let MyEffect::Time(_) = self { true } else { false }
            }
            pub fn into_time(
                self,
            ) -> Option<
                crux_core::Request<
                    <Time<MyEvent> as ::crux_core::capability::Capability<MyEvent>>::Operation,
                >,
            > {
                if let MyEffect::Time(request) = self { Some(request) } else { None }
            }
        }
        "###);
    }

    #[test]
    #[should_panic]
    fn should_panic_when_multiple_event_types() {
        let input = r#"
            #[derive(Effect)]
            pub struct Capabilities {
                pub render: Render<MyEvent>,
                pub time: Time<YourEvent>,
            }
        "#;
        let input = parse_str(input).unwrap();
        let input = EffectStructReceiver::from_derive_input(&input).unwrap();

        let mut actual = quote!();
        input.to_tokens(&mut actual);
    }

    fn pretty_print(ts: &proc_macro2::TokenStream) -> String {
        let file = syn::parse_file(&ts.to_string()).unwrap();
        prettyplease::unparse(&file)
    }

    #[test]
    fn split_event_types_preserves_path() {
        let ty = Type::from_string("crux_core::render::Render<Event>").unwrap();

        let (actual_type, actual_ident, actual_event) = split_on_generic(&ty);

        assert_eq!(
            quote!(#actual_type).to_string(),
            quote!(crux_core::render::Render).to_string()
        );

        assert_eq!(
            quote!(#actual_ident).to_string(),
            quote!(Render).to_string()
        );

        assert_eq!(quote!(#actual_event).to_string(), quote!(Event).to_string());
    }
}
