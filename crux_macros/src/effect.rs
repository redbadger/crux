use convert_case::{Case, Casing};
use darling::{ast, util, FromDeriveInput, FromField, FromMeta, ToTokens};
use proc_macro2::TokenStream;
use quote::quote;
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
#[darling(attributes(effect))]
pub struct EffectFieldReceiver {
    ident: Option<Ident>,
    ty: Type,
}

impl ToTokens for EffectStructReceiver {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ident = &self.ident;

        let name = match self.name {
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

        let event = extract_event_type(&fields.first().as_ref().unwrap().ty);
        let event = fields
            .iter()
            .map(|f| extract_event_type(&f.ty))
            .all(|t| quote!(#t).to_string() == quote!(#event).to_string())
            .then_some(event)
            .expect("all fields should be generic over the same event type");

        let (variants, fields): (Vec<_>, Vec<_>) = fields
            .into_iter()
            .map(|field| {
                let (field_name, variant) = field
                    .ident
                    .as_ref()
                    .map(|snake| {
                        let pascal =
                            Ident::new(&snake.to_string().to_case(Case::Pascal), snake.span());
                        (quote!(#snake), quote!(#pascal))
                    })
                    .expect("We already told darling we're on a struct with named fields");

                let ty = &field.ty;
                (
                    quote! { #variant(<#ty as ::crux_core::capability::Capability<#event>>::Operation) },
                    // TODO: Make this use the actual type, not #variant
                    quote! { #field_name: #variant::new(context.with_effect(#name::#variant)) },
                )
            })
            .unzip();

        tokens.extend(quote! {
            #[derive(Clone, ::serde::Serialize, ::serde::Deserialize, Debug, PartialEq, Eq)]
            pub enum #name {
                #(#variants ,)*
            }

            impl ::crux_core::WithContext<#app, #name> for #ident {
                fn new_with_context(context: ::crux_core::capability::CapabilityContext<#name, #event>) -> #ident {
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

fn extract_event_type(ty: &Type) -> Type {
    match ty {
        Type::Path(p) if p.qself.is_none() => {
            // Get the first segment of the path (there should be only one)
            let type_params = &p.path.segments.first().unwrap().arguments;
            // It should have only one angle-bracketed param
            let generic_arg = match type_params {
                PathArguments::AngleBracketed(params) => params.args.first(),
                _ => None,
            };
            // This argument must be a type
            match generic_arg {
                Some(GenericArgument::Type(ty)) => Some(ty.clone()),
                _ => None,
            }
        }
        _ => None,
    }
    .expect("capabilities should be generic over a single event type")
}

#[cfg(test)]
mod tests {
    use darling::{FromDeriveInput, ToTokens};
    use quote::quote;
    use syn::parse_str;

    use crate::effect::EffectStructReceiver;

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

        let mut actual = quote!();
        input.to_tokens(&mut actual);

        let expected = quote! {
            #[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
            pub enum Effect {
                Render,
            }

            impl crux_core::WithContext<App, Effect> for Capabilities {
                fn new_with_context(context: CapabilityContext<Effect, Event>) -> Capabilities {
                    Capabilities {
                        render: Render::new(context.with_effect(|_| Effect::Render)),
                    }
                }
            }
        };

        assert_eq!(actual.to_string(), expected.to_string());
    }

    #[test]
    fn full() {
        let input = r#"
            #[derive(Effect)]
            #[effect(name = "MyEffect", app = "MyApp")]
            pub struct MyCapabilities {
                #[effect(operation = "HttpRequest")]
                pub http: Http<MyEvent>,
                #[effect(operation = "KeyValueOperation")]
                pub key_value: KeyValue<MyEvent>,
                pub platform: Platform<MyEvent>,
                pub render: Render<MyEvent>,
                pub time: Time<MyEvent>,
            }
        "#;
        let input = parse_str(input).unwrap();
        let input = EffectStructReceiver::from_derive_input(&input).unwrap();

        let mut actual = quote!();
        input.to_tokens(&mut actual);

        let expected = quote! {
            #[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
            pub enum MyEffect {
                Http(HttpRequest),
                KeyValue(KeyValueOperation),
                Platform,
                Render,
                Time,
            }

            impl crux_core::WithContext<MyApp, MyEffect> for MyCapabilities {
                fn new_with_context(context: CapabilityContext<MyEffect, MyEvent>) -> MyCapabilities {
                    MyCapabilities {
                        http: Http::new(context.with_effect(MyEffect::Http)),
                        key_value: KeyValue::new(context.with_effect(MyEffect::KeyValue)),
                        platform: Platform::new(context.with_effect(|_| MyEffect::Platform)),
                        render: Render::new(context.with_effect(|_| MyEffect::Render)),
                        time: Time::new(context.with_effect(|_| MyEffect::Time)),
                    }
                }
            }
        };

        assert_eq!(actual.to_string(), expected.to_string());
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
}
