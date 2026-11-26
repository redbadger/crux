use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields};

pub fn derive(input: &DeriveInput) -> TokenStream {
    let name = &input.ident;
    let body = match &input.data {
        Data::Enum(data) => {
            let arms = data.variants.iter().map(|variant| {
                let ident = &variant.ident;
                let label = ident.to_string();
                let fields = match &variant.fields {
                    Fields::Unit => quote!(),
                    Fields::Unnamed(_) => quote!((..)),
                    Fields::Named(_) => quote!({ .. }),
                };
                quote!(Self::#ident #fields => #label)
            });
            quote!(match self { #(#arms,)* })
        }
        Data::Struct(_) => {
            let label = name.to_string();
            quote!(#label)
        }
        Data::Union(_) => {
            return syn::Error::new_spanned(name, "EventName cannot be derived for unions")
                .to_compile_error();
        }
    };
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();
    quote! {
        impl #impl_generics ::crux_core::EventName for #name #type_generics #where_clause {
            fn event_name(&self) -> &'static str { #body }
        }
    }
}
