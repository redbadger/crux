use heck::AsSnakeCase;
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::{Ident, ItemEnum, Type};

struct Effect {
    ident: Ident,
    operation: Type,
}

pub fn effect_impl(input: ItemEnum) -> TokenStream {
    let enum_ident = input.ident.clone();
    let enum_ident_str = enum_ident.to_string();

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

    let filters = effects.clone().map(|effect| {
        let effect_ident = &effect.ident;
        let effect_ident_str = effect.ident.to_string();
        let effect_ident_snake = AsSnakeCase(&effect_ident_str);
        let operation = &effect.operation;
        let filter_fn = Ident::new(&format!("is_{}", effect_ident_snake), Span::call_site());
        let map_fn = Ident::new(&format!("into_{}", effect_ident_snake), Span::call_site());
        let expect_fn = Ident::new(&format!("expect_{}", effect_ident_snake), Span::call_site());
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

    let type_gen = effects.clone().map(|effect| {
        let operation = &effect.operation;

        quote! {
            #operation::register_types(generator)?;
        }
    });

    quote! {
        #[derive(Debug)]
        pub enum #enum_ident {
            #(#effect_variants ,)*
        }

        #[derive(::serde::Serialize, ::serde::Deserialize)]
        #[serde(rename = #enum_ident_str)]
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

        #[cfg(feature = "typegen")]
        impl crux_core::typegen::Export for Effect {
            fn register_types(generator: &mut ::crux_core::typegen::TypeGen) -> ::crux_core::typegen::Result {
                use ::crux_core::capability::{Capability, Operation};
                #(#type_gen)*
                generator.register_type::<#ffi_enum_ident>()?;
                generator.register_type::<::crux_core::bridge::Request<#ffi_enum_ident>>()?;

                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod test {
    use syn::parse_quote;

    use super::*;

    #[test]
    fn single() {
        let input = parse_quote! {
            pub enum Effect {
                Render(RenderOperation),
            }
        };

        let actual = effect_impl(input);

        insta::assert_snapshot!(pretty_print(&actual), @r##"
        #[derive(Debug)]
        pub enum Effect {
            Render(::crux_core::Request<RenderOperation>),
        }
        #[derive(::serde::Serialize, ::serde::Deserialize)]
        #[serde(rename = "Effect")]
        pub enum EffectFfi {
            Render(RenderOperation),
        }
        impl crux_core::Effect for Effect {
            type Ffi = EffectFfi;
            fn serialize(self) -> (Self::Ffi, crux_core::bridge::ResolveSerialized) {
                match self {
                    Effect::Render(request) => request.serialize(EffectFfi::Render),
                }
            }
        }
        impl From<::crux_core::Request<RenderOperation>> for Effect {
            fn from(value: ::crux_core::Request<RenderOperation>) -> Self {
                Self::Render(value)
            }
        }
        impl Effect {
            pub fn is_render(&self) -> bool {
                if let Effect::Render(_) = self { true } else { false }
            }
            pub fn into_render(self) -> Option<::crux_core::Request<RenderOperation>> {
                if let Effect::Render(request) = self { Some(request) } else { None }
            }
            #[track_caller]
            pub fn expect_render(self) -> ::crux_core::Request<RenderOperation> {
                if let Effect::Render(request) = self {
                    request
                } else {
                    panic!("not a {} effect", "Render")
                }
            }
        }
        #[cfg(feature = "typegen")]
        impl crux_core::typegen::Export for Effect {
            fn register_types(
                generator: &mut ::crux_core::typegen::TypeGen,
            ) -> ::crux_core::typegen::Result {
                use ::crux_core::capability::{Capability, Operation};
                RenderOperation::register_types(generator)?;
                generator.register_type::<EffectFfi>()?;
                generator.register_type::<::crux_core::bridge::Request<EffectFfi>>()?;
                Ok(())
            }
        }
        "##);
    }

    #[test]
    fn multiple() {
        let input = parse_quote! {
            pub enum Effect {
                Render(RenderOperation),
                Http(HttpRequest),
            }
        };

        let actual = effect_impl(input);

        insta::assert_snapshot!(pretty_print(&actual), @r##"
        #[derive(Debug)]
        pub enum Effect {
            Render(::crux_core::Request<RenderOperation>),
            Http(::crux_core::Request<HttpRequest>),
        }
        #[derive(::serde::Serialize, ::serde::Deserialize)]
        #[serde(rename = "Effect")]
        pub enum EffectFfi {
            Render(RenderOperation),
            Http(HttpRequest),
        }
        impl crux_core::Effect for Effect {
            type Ffi = EffectFfi;
            fn serialize(self) -> (Self::Ffi, crux_core::bridge::ResolveSerialized) {
                match self {
                    Effect::Render(request) => request.serialize(EffectFfi::Render),
                    Effect::Http(request) => request.serialize(EffectFfi::Http),
                }
            }
        }
        impl From<::crux_core::Request<RenderOperation>> for Effect {
            fn from(value: ::crux_core::Request<RenderOperation>) -> Self {
                Self::Render(value)
            }
        }
        impl From<::crux_core::Request<HttpRequest>> for Effect {
            fn from(value: ::crux_core::Request<HttpRequest>) -> Self {
                Self::Http(value)
            }
        }
        impl Effect {
            pub fn is_render(&self) -> bool {
                if let Effect::Render(_) = self { true } else { false }
            }
            pub fn into_render(self) -> Option<::crux_core::Request<RenderOperation>> {
                if let Effect::Render(request) = self { Some(request) } else { None }
            }
            #[track_caller]
            pub fn expect_render(self) -> ::crux_core::Request<RenderOperation> {
                if let Effect::Render(request) = self {
                    request
                } else {
                    panic!("not a {} effect", "Render")
                }
            }
        }
        impl Effect {
            pub fn is_http(&self) -> bool {
                if let Effect::Http(_) = self { true } else { false }
            }
            pub fn into_http(self) -> Option<::crux_core::Request<HttpRequest>> {
                if let Effect::Http(request) = self { Some(request) } else { None }
            }
            #[track_caller]
            pub fn expect_http(self) -> ::crux_core::Request<HttpRequest> {
                if let Effect::Http(request) = self {
                    request
                } else {
                    panic!("not a {} effect", "Http")
                }
            }
        }
        #[cfg(feature = "typegen")]
        impl crux_core::typegen::Export for Effect {
            fn register_types(
                generator: &mut ::crux_core::typegen::TypeGen,
            ) -> ::crux_core::typegen::Result {
                use ::crux_core::capability::{Capability, Operation};
                RenderOperation::register_types(generator)?;
                HttpRequest::register_types(generator)?;
                generator.register_type::<EffectFfi>()?;
                generator.register_type::<::crux_core::bridge::Request<EffectFfi>>()?;
                Ok(())
            }
        }
        "##);
    }

    fn pretty_print(ts: &proc_macro2::TokenStream) -> String {
        let file = syn::parse_file(&ts.to_string()).unwrap();
        prettyplease::unparse(&file)
    }
}
