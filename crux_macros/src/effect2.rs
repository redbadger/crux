use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Ident, ItemEnum, Type};

struct Effect {
    ident: Ident,
    operation: Type,
}

pub fn effect2_macro_impl(tokens: TokenStream) -> TokenStream {
    let input = parse_macro_input!(tokens as ItemEnum);

    let enum_ident = input.ident.clone();

    let mut ffi_enum = input.clone();
    ffi_enum.ident = format_ident!("{}Ffi", enum_ident);
    let ffi_enum_ident = &ffi_enum.ident;

    let effects = input.variants.iter().map(|v| {
        let ident = v.ident.clone();
        let first_field = v.fields.iter().next().unwrap();
        let operation = first_field.ty.clone();
        Effect { ident, operation }
    });

    let effect_variants = effects.clone().map(|effect| {
        let effect_ident = &effect.ident;
        let operation = &effect.operation;
        quote! {
            #effect_ident(::crux_core::Request<#operation>)
        }
    });

    let match_arms = effects.clone().map(|effect| {
        let effect_ident = &effect.ident;
        quote! {
            #enum_ident::#effect_ident(request) => request.serialize(#ffi_enum_ident::#effect_ident)
        }
    });

    let from_impls = effects.clone().map(|effect| {
        let effect_ident = &effect.ident;
        let operation = &effect.operation;
        quote! {
            impl From<::crux_core::Request<#operation>> for #enum_ident {
                fn from(value: ::crux_core::Request<#operation>) -> Self {
                    Self::#effect_ident(value)
                }
            }
        }
    });

    quote! {
        #[derive(Debug)]
        pub enum #enum_ident {
            #(#effect_variants ,)*
        }

        #[derive(::serde::Serialize, ::serde::Deserialize)]
        #ffi_enum

        impl crux_core::Effect for #enum_ident {
            type Ffi = #ffi_enum_ident;
            fn serialize(self) -> (Self::Ffi, crux_core::bridge::ResolveSerialized) {
                match self {
                    #(#match_arms ,)*
                }
            }
        }

        #(#from_impls)*
    }
    .into()
}
