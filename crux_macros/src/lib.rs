use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, FieldsNamed};

/// Derive macro generating an impl of the trait Capabilities.
#[proc_macro_derive(Capabilities)]
pub fn derive_capabilities(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(input);

    let output = impl_derive_capabilities(&ident, &data);

    proc_macro::TokenStream::from(output)
}

fn impl_derive_capabilities(ident: &syn::Ident, data: &syn::Data) -> TokenStream {
    let outputs: Vec<TokenStream> = match data {
        syn::Data::Struct(s) => match &s.fields {
            syn::Fields::Named(FieldsNamed { named, .. }) => named
                .iter()
                .map(|f| {
                    let name = &f.ident;
                    let ty = &f.ty;
                    quote! {
                        impl crux_core::Capabilities<#ty> for #ident {
                            fn get(&self) -> &#ty {
                                &self.#name
                            }
                        }
                    }
                })
                .collect::<Vec<_>>(),
            _ => vec![],
        },
        _ => vec![],
    };

    quote! {
        #(#outputs)*
    }
}
