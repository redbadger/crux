use heck::ToSnakeCase;
use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Ident, ItemEnum, Token, Type};

struct Effect {
    ident: Ident,
    operation: Type,
    is_notification: bool,
}

/// Check if a variant has `#[effect(notification)]` attribute
fn has_notification_attr(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if attr.path().is_ident("effect")
            && let Ok(nested) = attr.parse_args::<syn::Ident>()
        {
            return nested == "notification";
        }
        false
    })
}

enum TypegenKind {
    Serde,
    Facet,
    None,
}

pub struct EffectArgs {
    typegen_kind: TypegenKind,
    native_bridge: bool,
}

#[cfg(test)]
impl EffectArgs {
    pub fn none() -> Self {
        Self {
            typegen_kind: TypegenKind::None,
            native_bridge: false,
        }
    }

    pub fn typegen() -> Self {
        Self {
            typegen_kind: TypegenKind::Serde,
            native_bridge: false,
        }
    }

    pub fn facet_typegen() -> Self {
        Self {
            typegen_kind: TypegenKind::Facet,
            native_bridge: false,
        }
    }

    pub fn with_native_bridge(mut self) -> Self {
        self.native_bridge = true;
        self
    }
}

impl Parse for EffectArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut typegen_kind = TypegenKind::None;
        let mut native_bridge = false;

        if !input.is_empty() {
            let idents = Punctuated::<Ident, Token![,]>::parse_terminated(input)?;
            for ident in &idents {
                if ident == "typegen" {
                    typegen_kind = TypegenKind::Serde;
                } else if ident == "facet_typegen" {
                    typegen_kind = TypegenKind::Facet;
                } else if ident == "native_bridge" {
                    native_bridge = true;
                } else {
                    panic!(
                        "Unexpected attribute: {ident}, expected typegen, facet_typegen, or native_bridge"
                    );
                }
            }
        }

        Ok(EffectArgs {
            typegen_kind,
            native_bridge,
        })
    }
}

#[allow(clippy::too_many_lines)]
pub fn effect_impl(args: EffectArgs, input: ItemEnum) -> TokenStream {
    let attrs = &input.attrs;
    let enum_ident = &input.ident;
    let EffectArgs {
        typegen_kind,
        native_bridge,
    } = args;
    let enum_ident_str = enum_ident.to_string();

    // Separate facet attributes from other attributes
    let (facet_attrs, non_facet_attrs): (Vec<_>, Vec<_>) =
        attrs.iter().partition(|attr| attr.path().is_ident("facet"));

    let mut ffi_enum = input.clone();
    ffi_enum.ident = format_ident!("{}Ffi", enum_ident);
    ffi_enum.attrs = vec![];
    // Strip #[effect(notification)] attributes from variants
    for variant in &mut ffi_enum.variants {
        variant.attrs.retain(|attr| !attr.path().is_ident("effect"));
    }
    let ffi_enum_ident = &ffi_enum.ident;

    let ffi_enum = match typegen_kind {
        TypegenKind::Serde => quote! {
            #[derive(::serde::Serialize, ::serde::Deserialize)]
            #[serde(rename = #enum_ident_str)]
            #ffi_enum
        },
        TypegenKind::Facet => {
            let facet_meta_attrs = facet_attrs.iter().map(|attr| &attr.meta);

            let native_bridge_derive = if native_bridge {
                quote! {
                    #[cfg_attr(feature = "native_bridge", derive(::uniffi::Enum))]
                }
            } else {
                quote! {}
            };

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
                #native_bridge_derive
                #ffi_enum
            }
        }
        TypegenKind::None if native_bridge => quote! {
            #[derive(::serde::Serialize, ::serde::Deserialize)]
            #[serde(rename = #enum_ident_str)]
            #[cfg_attr(feature = "native_bridge", derive(::uniffi::Enum))]
            #ffi_enum
        },
        TypegenKind::None => quote! {},
    };

    let effects = input.variants.into_iter().map(|variant| {
        let ident = variant.ident;
        let is_notification = has_notification_attr(&variant.attrs);
        let operation = variant
            .fields
            .into_iter()
            .next()
            .expect("each variant is expected to be a tuple with one field")
            .ty;
        Effect {
            ident,
            operation,
            is_notification,
        }
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

    // Generate native bridge types (EffectOutput, NativeRequest, EffectNative impl)
    // Only when native_bridge is explicitly requested via macro argument
    let native_bridge_code = if native_bridge {
        let effect_output_variants = effects.clone().map(|effect| {
            let effect_ident = &effect.ident;
            let operation = &effect.operation;

            if effect.is_notification {
                // Unit variant for notification/fire-and-forget effects
                quote! { #effect_ident }
            } else {
                quote! {
                    #effect_ident(<#operation as ::crux_core::capability::Operation>::Output)
                }
            }
        });

        let native_match_arms = effects.clone().map(|effect| {
            let effect_ident = &effect.ident;
            let effect_ident_str = effect.ident.to_string();

            if effect.is_notification {
                // Notification effects extract () from unit variant
                quote! {
                    #enum_ident::#effect_ident(req) => req.into_native(
                        #ffi_enum_ident::#effect_ident,
                        |o| match o {
                            EffectOutput::#effect_ident => Ok(()),
                            _ => Err(::crux_core::bridge::NativeBridgeError::OutputMismatch {
                                expected: #effect_ident_str.to_string(),
                            }),
                        },
                    )
                }
            } else {
                quote! {
                    #enum_ident::#effect_ident(req) => req.into_native(
                        #ffi_enum_ident::#effect_ident,
                        |o| match o {
                            EffectOutput::#effect_ident(v) => Ok(v),
                            _ => Err(::crux_core::bridge::NativeBridgeError::OutputMismatch {
                                expected: #effect_ident_str.to_string(),
                            }),
                        },
                    )
                }
            }
        });

        let effect_output_derive = quote! {
            #[cfg_attr(feature = "native_bridge", derive(::uniffi::Enum))]
        };
        let native_request_derive = quote! {
            #[cfg_attr(feature = "native_bridge", derive(::uniffi::Record))]
        };

        quote! {
            #[cfg(feature = "native_bridge")]
            #effect_output_derive
            pub enum EffectOutput {
                #(#effect_output_variants ,)*
            }

            #[cfg(feature = "native_bridge")]
            #native_request_derive
            pub struct NativeRequest {
                pub id: u32,
                pub effect: #ffi_enum_ident,
            }

            #[cfg(feature = "native_bridge")]
            impl ::crux_core::EffectNative for #enum_ident {
                type Ffi = #ffi_enum_ident;
                type Output = EffectOutput;

                fn into_native(self) -> (Self::Ffi, ::crux_core::bridge::ResolveNative<Self::Output>) {
                    match self {
                        #(#native_match_arms ,)*
                    }
                }
            }
        }
    } else {
        quote! {}
    };

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

    let has_ffi = !matches!(typegen_kind, TypegenKind::None) || native_bridge;
    let effect_ffi_derive = if has_ffi {
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
    } else {
        quote! {}
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

        #native_bridge_code

        #(#from_impls)*

        #(#filters)*

        #type_gen

    }
}
