use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse::Parse, ItemStruct, Type};

pub(crate) struct GenerateEffectAttr {
    pub ty: Type,
}

impl Parse for GenerateEffectAttr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ty = input.parse()?;
        Ok(GenerateEffectAttr { ty })
    }
}

pub(crate) fn impl_generate_effect(attr: GenerateEffectAttr, item: ItemStruct) -> TokenStream {
    let effect_type = attr.ty;
    let ident = &item.ident;
    quote! {
        #item

        #[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
        pub enum #effect_type {
            Http(HttpRequest),
            Render,
        }

        impl crux_core::WithContext<App, #effect_type> for #ident {
            fn new_with_context(context: CapabilityContext<#effect_type, Event>) -> #ident {
                #ident {
                    http: Http::new(context.with_effect(#effect_type::Http)),
                    render: Render::new(context.with_effect(|_| #effect_type::Render)),
                }
            }
        }
    }
}
