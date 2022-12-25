use convert_case::{Case, Casing};
use darling::{ast, util, FromDeriveInput, FromField, FromMeta};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Ident, Index, Type};

#[derive(FromDeriveInput)]
#[darling(attributes(effect), supports(struct_named))]
struct EffectStructReceiver {
    ident: Ident,
    name: Option<Type>,
    app: Option<Type>,
    event: Option<Type>,
    data: ast::Data<util::Ignored, EffectFieldReceiver>,
}

#[derive(Debug, FromField)]
#[darling(attributes(effect))]
pub struct EffectFieldReceiver {
    ident: Option<Ident>,
    operation: Option<Type>,
}

pub(crate) fn effect_impl(input: &DeriveInput) -> TokenStream {
    let input = match EffectStructReceiver::from_derive_input(input) {
        Ok(v) => v,
        Err(e) => {
            return e.write_errors();
        }
    };

    let ident = &input.ident;
    let name = input
        .name
        .unwrap_or_else(|| Type::from_string("Effect").unwrap());

    let app = input
        .app
        .unwrap_or_else(|| Type::from_string("App").unwrap());

    let event = input
        .event
        .unwrap_or_else(|| Type::from_string("Event").unwrap());

    let fields = input
        .data
        .as_ref()
        .take_struct()
        .expect("Should never be enum")
        .fields;

    let (variants, fields): (Vec<_>, Vec<_>) = fields
        .into_iter()
        .enumerate()
        .map(|(i, f)| {
            // This works with named or indexed fields, so we'll fall back to the index so we can
            // write the output as a key-value pair.
            let (snake, pascal) = f
                .ident
                .as_ref()
                .map(|snake| {
                    let pascal = Ident::new(&snake.to_string().to_case(Case::Pascal), snake.span());
                    (quote!(#snake), quote!(#pascal))
                })
                .unwrap_or_else(|| {
                    let i = Index::from(i);
                    (quote!(#i), quote!(#i))
                });

            if let Some(operation) = &f.operation {
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

    quote! {
        #[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
        pub enum #name {
            #(#variants),*
        }

        impl crux_core::WithContext<#app, #name> for #ident {
            fn new_with_context(context: CapabilityContext<#name, #event>) -> #ident {
                #ident {
                    #(#fields),*
                }
            }
        }
    }
}
