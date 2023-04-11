use darling::{ast, util, FromDeriveInput, FromField, FromMeta, ToTokens};
use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};
use std::collections::BTreeMap;
use syn::{DeriveInput, GenericArgument, Ident, PathArguments, Type};

#[derive(FromDeriveInput, Debug)]
#[darling(attributes(effect), supports(struct_named))]
struct EffectStructReceiver {
    ident: Ident,
    name: Option<Ident>,
    app: Option<Type>,
    data: ast::Data<util::Ignored, EffectFieldReceiver>,
}

#[derive(FromField, Debug)]
pub struct EffectFieldReceiver {
    ident: Option<Ident>,
    ty: Type,
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

        let app = match self.app {
            Some(ref app) => quote!(#app),
            None => {
                let x = Type::from_string("App").unwrap();
                quote!(#x)
            }
        };

        let fields = self
            .data
            .as_ref()
            .take_struct()
            .expect("Should never be enum")
            .fields;

        let fields: BTreeMap<Ident, (Type, Ident, Type)> = fields
            .iter()
            .map(|f| (f.ident.clone().unwrap(), split_on_generic(&f.ty)))
            .collect();

        let events: Vec<_> = fields.values().map(|(_, _, t)| t).collect();
        if !events
            .windows(2)
            .all(|win| win[0].to_token_stream().to_string() == win[1].to_token_stream().to_string())
        {
            panic!("all fields should be generic over the same event type");
        }
        let event = events.first().expect("Capabilities struct has no fields");

        let mut variants = Vec::new();
        let mut with_context_fields = Vec::new();
        let mut ffi_variants = Vec::new();
        let mut match_arms = Vec::new();

        for (field_name, (capability, variant, event)) in fields.iter() {
            variants.push(quote! { #variant(::crux_core::Request<<#capability<#event> as ::crux_core::capability::Capability<#event>>::Operation>) });
            with_context_fields.push(quote! { #field_name: #capability::new(context.specialize(#effect_name::#variant)) });
            ffi_variants.push(quote! { #variant(<#capability<#event> as ::crux_core::capability::Capability<#event>>::Operation) });
            match_arms.push(quote! { #effect_name::#variant(request) => request.serialize(#ffi_effect_name::#variant) });
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

                fn serialize<'out>(self) -> (Self::Ffi, crux_core::bridge::ResolveBytes) {
                    match self {
                        #(#match_arms ,)*
                    }
                }
            }

            impl ::crux_core::WithContext<#app, #effect_name> for #ident {
                fn new_with_context(context: ::crux_core::capability::ProtoContext<#effect_name, #event>) -> #ident {
                    #ident {
                        #(#with_context_fields ,)*
                    }
                }
            }
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

fn split_on_generic(ty: &Type) -> (Type, Ident, Type) {
    let ty = ty.clone();
    match ty {
        Type::Path(mut path) if path.qself.is_none() => {
            // Get the last segment of the path where the generic parameter should be

            let last = path.path.segments.last_mut().expect("type has no segments");
            let type_name = last.ident.clone();
            let type_params = std::mem::take(&mut last.arguments);

            // It should have only one angle-bracketed param
            let generic_arg = match type_params {
                PathArguments::AngleBracketed(params) => params.args.first().cloned(),
                _ => None,
            };

            // This argument must be a type
            match generic_arg {
                Some(GenericArgument::Type(t2)) => Some((Type::Path(path), type_name, t2)),
                _ => None,
            }
        }
        _ => None,
    }
    .expect("capabilities should be generic over a single event type")
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
            fn serialize<'out>(self) -> (Self::Ffi, crux_core::bridge::ResolveBytes) {
                match self {
                    Effect::Render(request) => request.serialize(EffectFfi::Render),
                }
            }
        }
        impl ::crux_core::WithContext<App, Effect> for Capabilities {
            fn new_with_context(
                context: ::crux_core::capability::ProtoContext<Effect, Event>,
            ) -> Capabilities {
                Capabilities {
                    render: Render::new(context.specialize(Effect::Render)),
                }
            }
        }
        "###);
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

    #[test]
    fn full() {
        let input = r#"
            #[derive(Effect)]
            #[effect(name = "MyEffect", app = "MyApp")]
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
            fn serialize<'out>(self) -> (Self::Ffi, crux_core::bridge::ResolveBytes) {
                match self {
                    MyEffect::Http(request) => request.serialize(MyEffectFfi::Http),
                    MyEffect::KeyValue(request) => request.serialize(MyEffectFfi::KeyValue),
                    MyEffect::Platform(request) => request.serialize(MyEffectFfi::Platform),
                    MyEffect::Render(request) => request.serialize(MyEffectFfi::Render),
                    MyEffect::Time(request) => request.serialize(MyEffectFfi::Time),
                }
            }
        }
        impl ::crux_core::WithContext<MyApp, MyEffect> for MyCapabilities {
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
}
