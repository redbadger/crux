use heck::ToSnakeCase;
use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, format_ident, quote};
use syn::{Ident, ItemEnum, Type};

struct Effect {
    ident: Ident,
    operation: Type,
}

enum TypegenKind {
    Serde,
    Facet,
    None,
}

impl From<Option<Ident>> for TypegenKind {
    fn from(value: Option<Ident>) -> Self {
        match value {
            Some(x) if x == format_ident!("typegen") => TypegenKind::Serde,
            Some(x) if x == format_ident!("facet_typegen") => TypegenKind::Facet,
            Some(x) => panic!("Unexpected attribute: {x}, did you mean typegen or facet_typegen?"),
            None => TypegenKind::None,
        }
    }
}

#[allow(clippy::too_many_lines)]
pub fn effect_impl(args: Option<Ident>, input: ItemEnum) -> TokenStream {
    let attrs = &input.attrs;
    let enum_ident = &input.ident;
    let typegen_kind: TypegenKind = args.into();
    let enum_ident_str = enum_ident.to_string();

    // Separate facet attributes from other attributes
    let (facet_attrs, non_facet_attrs): (Vec<_>, Vec<_>) =
        attrs.iter().partition(|attr| attr.path().is_ident("facet"));

    let mut ffi_enum = input.clone();
    ffi_enum.ident = format_ident!("{}Ffi", enum_ident);
    ffi_enum.attrs = vec![];
    let ffi_enum_ident = &ffi_enum.ident;

    let ffi_enum = match typegen_kind {
        TypegenKind::Serde => quote! {
            #[derive(::serde::Serialize, ::serde::Deserialize)]
            #[serde(rename = #enum_ident_str)]
            #ffi_enum
        },
        TypegenKind::Facet => {
            let facet_meta_attrs = facet_attrs.iter().map(|attr| &attr.meta);

            quote! {
                #[derive(::serde::Serialize, ::serde::Deserialize)]
                #[serde(rename = #enum_ident_str)]
                #[cfg_attr(
                    feature = "facet_typegen",
                    derive(::facet::Facet),
                    #(#facet_meta_attrs,)*
                    facet(name = #enum_ident_str),
                    repr(C)
                )]
                #ffi_enum
            }
        }
        TypegenKind::None => quote! {},
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

    let type_gen = match typegen_kind {
        TypegenKind::Serde => {
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
                        use ::crux_core::capability::Operation;
                        #(#effect_gen)*
                        generator.register_type::<#ffi_enum_ident>()?;
                        generator.register_type::<::crux_core::bridge::Request<#ffi_enum_ident>>()?;

                        Ok(())
                    }
                }
            }
        }
        TypegenKind::Facet => {
            let effect_gen = effects.map(|effect| {
                let operation = &effect.operation;

                quote! {
                    let generator = #operation::register_types_facet(generator)
                        .map_err(|err| ::crux_core::type_generation::facet::TypeGenError::Generation(err.to_string()))?;
                }
            });
            quote! {
                #[cfg(feature = "facet_typegen")]
                impl ::crux_core::type_generation::facet::Export for #enum_ident {
                    fn register_types(
                        generator: &mut ::crux_core::type_generation::facet::TypeRegistry,
                    ) -> Result<&mut ::crux_core::type_generation::facet::TypeRegistry, ::crux_core::type_generation::facet::TypeGenError> {
                        use ::crux_core::capability::Operation;
                        #(#effect_gen)*
                        generator
                            .register_type::<#ffi_enum_ident>()
                            .map_err(|err| ::crux_core::type_generation::facet::TypeGenError::Generation(err.to_string()))?
                            .register_type::<::crux_core::bridge::Request<#ffi_enum_ident>>()
                            .map_err(|err| ::crux_core::type_generation::facet::TypeGenError::Generation(err.to_string()))?;

                        Ok(generator)
                    }
                }
            }
        }
        TypegenKind::None => quote! {},
    };

    let effect_ffi_derive = if let TypegenKind::None = typegen_kind {
        quote! {}
    } else {
        quote! {
            impl crux_core::EffectFFI for #enum_ident {
                type Ffi = #ffi_enum_ident;
                fn serialize<T: ::crux_core::bridge::FfiFormat>(self) -> (Self::Ffi, ::crux_core::bridge::ResolveSerialized<T>) {
                    match self {
                        #(#match_arms ,)*
                    }
                }
            }
        }
    };

    let attrs = if non_facet_attrs.is_empty() {
        quote! {}
    } else {
        let tokens = non_facet_attrs.iter().map(ToTokens::to_token_stream);
        quote! {
            #(#tokens)*
        }
    };

    let original_enum = quote! {
        #attrs
        pub enum #enum_ident {
            #(#effect_variants ,)*
        }
    };

    quote! {
        #original_enum

        #ffi_enum

        impl crux_core::Effect for #enum_ident {}

        #effect_ffi_derive

        #(#from_impls)*

        #(#filters)*

        #type_gen

    }
}
