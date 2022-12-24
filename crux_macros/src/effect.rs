use darling::{ast, util, FromDeriveInput, FromField, FromMeta};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Ident, Type};

#[derive(FromDeriveInput)]
#[darling(attributes(effect), supports(struct_named))]
struct EffectStructReceiver {
    ident: Ident,
    name: Option<Type>,
    data: ast::Data<util::Ignored, EffectFieldReceiver>,
}

#[derive(Debug, FromField)]
#[darling(attributes(effect))]
pub struct EffectFieldReceiver {
    ident: Option<Ident>,
    operation: Option<Type>,
}

pub(crate) fn effect_impl(input: &DeriveInput) -> TokenStream {
    let args = match EffectStructReceiver::from_derive_input(input) {
        Ok(v) => v,
        Err(e) => {
            return e.write_errors();
        }
    };

    let ident = &input.ident;
    let name = args
        .name
        .unwrap_or_else(|| Type::from_string("Effect").unwrap());

    quote! {
        #[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
        pub enum #name {
            Http(HttpRequest),
            Render,
        }

        impl crux_core::WithContext<App, #name> for #ident {
            fn new_with_context(context: CapabilityContext<#name, Event>) -> #ident {
                #ident {
                    http: Http::new(context.with_effect(#name::Http)),
                    render: Render::new(context.with_effect(|_| #name::Render)),
                }
            }
        }
    }
}
