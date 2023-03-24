use darling::{ast, util, FromDeriveInput, FromField, FromMeta, ToTokens};
use proc_macro2::TokenStream;
use quote::quote;
use std::collections::BTreeMap;
use syn::{DeriveInput, GenericArgument, Ident, PathArguments, Type};

#[derive(FromDeriveInput, Debug)]
#[darling(attributes(effect), supports(struct_named))]
struct EffectStructReceiver {
    ident: Ident,
    name: Option<Type>,
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

        let effect_name = match self.name {
            Some(ref name) => quote!(#name),
            None => {
                let x = Type::from_string("Effect").unwrap();
                quote!(#x)
            }
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

        let (variants, fields): (Vec<_>, Vec<_>) = fields.iter()
            .map(|(field_name, (ty, variant, event))| {
                (
                    quote! { #variant(::crux_core::steps::Step<<#ty<#event> as ::crux_core::capability::Capability<#event>>::Operation>) },
                    quote! { #field_name: #ty::new(context.specialise(#effect_name::#variant)) },
                )
            })
            .unzip();

        tokens.extend(quote! {
            #[derive(Debug, Serialize, PartialEq, Eq)]
            pub enum #effect_name {
                #(#variants ,)*
            }

            impl ::crux_core::WithContext<#app, #effect_name> for #ident {
                fn new_with_context(context: ::crux_core::capability::ProtoContext<#effect_name, #event>) -> #ident {
                    #ident {
                        #(#fields ,)*
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
        #[derive(Clone, ::serde::Serialize, Debug, PartialEq, Eq)]
        pub enum Effect {
            Render(
                ::crux_core::steps::Step<
                    <Render<Event> as ::crux_core::capability::Capability<Event>>::Operation,
                >,
            ),
        }
        impl ::crux_core::WithContext<App, Effect> for Capabilities {
            fn new_with_context(
                context: ::crux_core::capability::ProtoContext<Effect, Event>,
            ) -> Capabilities {
                Capabilities {
                    render: Render::new(context.specialise(Effect::Render)),
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
        #[derive(Clone, ::serde::Serialize, Debug, PartialEq, Eq)]
        pub enum MyEffect {
            Http(
                ::crux_core::steps::Step<
                    <crux_http::Http<
                        MyEvent,
                    > as ::crux_core::capability::Capability<MyEvent>>::Operation,
                >,
            ),
            KeyValue(
                ::crux_core::steps::Step<<KeyValue<MyEvent> as ::crux_core::capability::Capability<MyEvent>>::Operation>,
            ),
            Platform(
                ::crux_core::steps::Step<<Platform<MyEvent> as ::crux_core::capability::Capability<MyEvent>>::Operation>,
            ),
            Render(::crux_core::steps::Step<<Render<MyEvent> as ::crux_core::capability::Capability<MyEvent>>::Operation>),
            Time(::crux_core::steps::Step<<Time<MyEvent> as ::crux_core::capability::Capability<MyEvent>>::Operation>),
        }
        impl ::crux_core::WithContext<MyApp, MyEffect> for MyCapabilities {
            fn new_with_context(
                context: ::crux_core::capability::ProtoContext<MyEffect, MyEvent>,
            ) -> MyCapabilities {
                MyCapabilities {
                    http: crux_http::Http::new(context.specialise(MyEffect::Http)),
                    key_value: KeyValue::new(context.specialise(MyEffect::KeyValue)),
                    platform: Platform::new(context.specialise(MyEffect::Platform)),
                    render: Render::new(context.specialise(MyEffect::Render)),
                    time: Time::new(context.specialise(MyEffect::Time)),
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
