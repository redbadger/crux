use proc_macro2::TokenStream;
use quote::quote;
use std::collections::HashMap;
use syn::{parse::Parse, punctuated::Punctuated, ItemStruct, Token, Type};

pub(crate) struct GenerateEffectAttr {
    effect_type: Type,
    operations: Punctuated<Operation, Token![,]>,
}

type Operation = Punctuated<Type, Token!(=)>;

impl Parse for GenerateEffectAttr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let effect_type = input.parse()?;
        input.parse::<Token![,]>()?;
        let operations =
            Punctuated::parse_separated_nonempty_with(input, Punctuated::parse_separated_nonempty)?;
        Ok(GenerateEffectAttr {
            effect_type,
            operations,
        })
    }
}

pub(crate) fn impl_generate_effect(attr: GenerateEffectAttr, item: ItemStruct) -> TokenStream {
    let GenerateEffectAttr {
        effect_type,
        operations,
    } = attr;
    let ident = &item.ident;

    // for op in operations {
    //     let key = op.first();
    //     let val = op.last();
    // }

    // let lookup: HashMap<_, _> = operations
    //     .into_pairs()
    //     .map(|p| p.into_value().into_pairs())
    //     .map(|op| {
    //         let op = op.into_value();
    //         (op.first().clone(), op.last())
    //     })
    //     .collect();
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
