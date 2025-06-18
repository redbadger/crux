use heck::ToSnakeCase;
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::{Ident, ItemEnum, Type};

struct Effect {
    ident: Ident,
    operation: Type,
}

#[allow(clippy::too_many_lines)]
pub fn effect_impl(args: Option<Ident>, input: ItemEnum) -> TokenStream {
    let enum_ident = &input.ident;
    let has_typegen_attr = match args {
        Some(x) if x == format_ident!("typegen") => true,
        None => false,
        _ => panic!("did you mean typegen?"),
    };
    let enum_ident_str = enum_ident.to_string();

    let mut ffi_enum = input.clone();
    ffi_enum.ident = format_ident!("{}Ffi", enum_ident);
    ffi_enum.attrs = vec![];
    let ffi_enum_ident = &ffi_enum.ident;

    let ffi_enum = if cfg!(feature = "facet_typegen") {
        quote! {
            #[derive(::serde::Serialize, ::serde::Deserialize)]
            #[serde(rename = #enum_ident_str)]
            #[cfg_attr(feature = "facet_typegen", derive(::facet::Facet))]
            #[cfg_attr(feature = "facet_typegen", facet(rename = #enum_ident_str))]
            #[cfg_attr(feature = "facet_typegen", repr(C))]
            #ffi_enum
        }
    } else {
        quote! {
            #[derive(::serde::Serialize, ::serde::Deserialize)]
            #[serde(rename = #enum_ident_str)]
            #ffi_enum
        }
    };

    let effects = input.variants.into_iter().map(|variant| {
        let ident = variant.ident;
        let operation = variant
            .fields
            .into_iter()
            .next()
            .expect("each variant is expected to be a tuple with one field")
            .ty;
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

            impl TryFrom<#enum_ident> for ::crux_core::Request<#operation> {
                type Error = #enum_ident;

                fn try_from(value: #enum_ident) -> Result<Self, Self::Error> {
                    if let #enum_ident::#effect_ident(value) = value {
                        Ok(value)
                    } else {
                        Err(value)
                    }
                }
            }
        }
    });

    let filters = effects.clone().map(|effect| {
        let effect_ident = &effect.ident;
        let effect_ident_str = effect.ident.to_string();
        let effect_ident_snake = effect_ident_str.to_snake_case();
        let operation = &effect.operation;
        let filter_fn = Ident::new(&format!("is_{effect_ident_snake}"), Span::call_site());
        let map_fn = Ident::new(&format!("into_{effect_ident_snake}"), Span::call_site());
        let expect_fn = Ident::new(&format!("expect_{effect_ident_snake}"), Span::call_site());
        quote! {
            impl #enum_ident {
                pub fn #filter_fn(&self) -> bool {
                    if let #enum_ident::#effect_ident(_) = self {
                        true
                    } else {
                        false
                    }
                }
                pub fn #map_fn(self) -> Option<::crux_core::Request<#operation>> {
                    if let #enum_ident::#effect_ident(request) = self {
                        Some(request)
                    } else {
                        None
                    }
                }
                #[track_caller]
                pub fn #expect_fn(self) -> ::crux_core::Request<#operation> {
                    if let #enum_ident::#effect_ident(request) = self {
                        request
                    } else {
                        panic!("not a {} effect", #effect_ident_str)
                    }
                }
            }
        }
    });

    let type_gen = if has_typegen_attr {
        if cfg!(feature = "facet_typegen") {
            let effect_gen = effects.map(|effect| {
                let operation = &effect.operation;

                quote! {
                    #operation::register_types_facet(generator)?;
                }
            });
            quote! {
                #[cfg(feature = "facet_typegen")]
                impl ::crux_core::type_generation::facet::Export for #enum_ident {
                    fn register_types(
                        generator: &mut ::crux_core::type_generation::facet::TypeGen
                    ) -> ::crux_core::type_generation::facet::Result {
                        use ::crux_core::capability::{Capability, Operation};
                        #(#effect_gen)*
                        generator.register_type::<#ffi_enum_ident>()?;
                        generator.register_type::<::crux_core::bridge::Request<#ffi_enum_ident>>()?;

                        Ok(())
                    }
                }
            }
        } else {
            let effect_gen = effects.map(|effect| {
                let operation = &effect.operation;

                quote! {
                    #operation::register_types(generator)?;
                }
            });
            quote! {
                #[cfg(feature = "typegen")]
                impl ::crux_core::type_generation::serde::Export for #enum_ident {
                    fn register_types(
                        generator: &mut ::crux_core::type_generation::serde::TypeGen
                    ) -> ::crux_core::type_generation::serde::Result {
                        use ::crux_core::capability::{Capability, Operation};
                        #(#effect_gen)*
                        generator.register_type::<#ffi_enum_ident>()?;
                        generator.register_type::<::crux_core::bridge::Request<#ffi_enum_ident>>()?;

                        Ok(())
                    }
                }
            }
        }
    } else {
        quote! {}
    };

    quote! {
        #[derive(Debug)]
        pub enum #enum_ident {
            #(#effect_variants ,)*
        }

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

        #(#filters)*

        #type_gen

    }
}
