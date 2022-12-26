use convert_case::{Case, Casing};
use darling::{ast, util, FromDeriveInput, FromField, FromMeta, ToTokens};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Ident, Type};

#[derive(FromDeriveInput, Debug)]
#[darling(attributes(effect), supports(struct_named))]
struct EffectStructReceiver {
    ident: Ident,
    name: Option<Type>,
    app: Option<Type>,
    event: Option<Type>,
    data: ast::Data<util::Ignored, EffectFieldReceiver>,
}

#[derive(FromField, Debug)]
#[darling(attributes(effect))]
pub struct EffectFieldReceiver {
    ident: Option<Ident>,
    operation: Option<Type>,
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

        let event = match self.event {
            Some(ref event) => quote!(#event),
            None => {
                let x = Type::from_string("Event").unwrap();
                quote!(#x)
            }
        };

        let fields = self
            .data
            .as_ref()
            .take_struct()
            .expect("Should never be enum")
            .fields;

        let (variants, fields): (Vec<_>, Vec<_>) = fields
            .into_iter()
            .map(|field| {
                let (snake, pascal) = field
                    .ident
                    .as_ref()
                    .map(|snake| {
                        let pascal =
                            Ident::new(&snake.to_string().to_case(Case::Pascal), snake.span());
                        (quote!(#snake), quote!(#pascal))
                    })
                    .expect("We already told darling we're on a struct with named fields");

                if let Some(operation) = &field.operation {
                    (
                        quote! {#pascal(#operation)},
                        quote! {#snake: #pascal::new(context.with_effect(#name::#pascal))},
                    )
                } else {
                    (
                        quote! {#pascal},
                        quote! {#snake: #pascal::new(context.with_effect(|_| #name::#pascal))},
                    )
                }
            })
            .unzip();

        tokens.extend(quote! {
            #[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
            pub enum #name {
                #(#variants ,)*
            }

            impl crux_core::WithContext<#app, #name> for #ident {
                fn new_with_context(context: CapabilityContext<#name, #event>) -> #ident {
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
            #[effect(name = "MyEffect", app = "MyApp", event = "MyEvent")]
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
}
