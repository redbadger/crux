use proc_macro::TokenStream;
use quote::quote;
// use syn;

/// Derive macro generating an impl of the trait Capabilities.
#[proc_macro_derive(Capabilities)]
pub fn derive_capabilities(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).expect("could not parse the capabilities struct");

    impl_derive_capabilities(&ast)
}

fn impl_derive_capabilities(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let gen = quote! {
        impl crux_core::Capabilities<Http<Effect>> for #name {
            fn get(&self) -> &Http<Effect> {
                &self.http
            }
        }
    };
    gen.into()
}
