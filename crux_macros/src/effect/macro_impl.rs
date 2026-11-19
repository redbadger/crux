use heck::{ToSnakeCase, ToUpperCamelCase};
use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, format_ident, quote};
use syn::{Attribute, Ident, ItemEnum, LitStr, Type};

#[derive(Clone)]
struct Effect {
    ident: Ident,
    operation: Type,
    /// The handler group from `#[codegen(group = "Name")]` on the variant, or
    /// `None`.
    group: Option<String>,
}

/// The attribute that changes only the generated shell code.
const CODEGEN: &str = "codegen";

/// Reads `#[codegen(group = "Name")]` from the attributes of a variant.
///
/// The group name becomes part of a type name in each shell language
/// (`<Name>Handler`), so it must be in upper camel case.
fn variant_group(attrs: &[Attribute]) -> syn::Result<Option<String>> {
    let mut group: Option<String> = None;
    for attr in attrs.iter().filter(|attr| attr.path().is_ident(CODEGEN)) {
        attr.parse_nested_meta(|meta| {
            if !meta.path.is_ident("group") {
                let key = meta
                    .path
                    .get_ident()
                    .map_or_else(|| "?".to_string(), ToString::to_string);
                return Err(meta.error(format!(
                    "unknown key `{key}` in `#[codegen(...)]`, expected `group = \"Name\"`"
                )));
            }
            let value: LitStr = meta.value()?.parse()?;
            let name = value.value();
            if !is_upper_camel_case(&name) {
                let suggestion = Some(name.to_upper_camel_case())
                    .filter(|suggestion| is_upper_camel_case(suggestion))
                    .unwrap_or_else(|| "Store".to_string());
                return Err(syn::Error::new(
                    value.span(),
                    format!(
                        "the group name `{name}` is not in upper camel case. A group name becomes part of a type name in the shell (`<Name>Handler`), so use a name such as \"{suggestion}\""
                    ),
                ));
            }
            if group.is_some() {
                return Err(syn::Error::new(
                    value.span(),
                    "a variant can be in only one group",
                ));
            }
            group = Some(name);
            Ok(())
        })?;
    }
    Ok(group)
}

/// Whether `name` is an ASCII identifier in upper camel case, such as `Store`
/// or `Http2`. `HTTP` is not, because two group names that differ only in
/// case would give the same parameter name in the shell.
fn is_upper_camel_case(name: &str) -> bool {
    name.chars().next().is_some_and(|c| c.is_ascii_uppercase())
        && name.chars().all(|c| c.is_ascii_alphanumeric())
        && name == name.to_upper_camel_case()
}

/// The request id carries the effect's variant index in 8 bits.
pub const MAX_VARIANTS: usize = 256;

enum TypegenKind {
    Facet,
    None,
}

#[allow(clippy::fallible_impl_from)]
impl From<Option<Ident>> for TypegenKind {
    fn from(value: Option<Ident>) -> Self {
        match value {
            Some(x) if x == format_ident!("facet_typegen") => Self::Facet,
            Some(x) if x == format_ident!("typegen") => panic!(
                "`#[effect(typegen)]` (serde type generation) was removed in crux_core 0.21; use `#[effect(facet_typegen)]`"
            ),
            Some(x) => panic!("Unexpected attribute: {x}, did you mean facet_typegen?"),
            None => Self::None,
        }
    }
}

#[allow(clippy::too_many_lines)]
pub fn effect_impl(args: Option<Ident>, input: ItemEnum) -> TokenStream {
    let attrs = &input.attrs;
    let enum_ident = &input.ident;
    let typegen_kind: TypegenKind = args.into();
    let enum_ident_str = enum_ident.to_string();

    let mut errors: Vec<TokenStream> = Vec::new();

    // `#[codegen(...)]` belongs on a variant. On the enum, rustc would report
    // an unknown attribute, so report something clearer and drop it.
    for attr in attrs.iter().filter(|attr| attr.path().is_ident(CODEGEN)) {
        errors.push(
            syn::Error::new_spanned(
                attr,
                "`#[codegen(...)]` goes on a variant of the effect enum, not on the enum",
            )
            .to_compile_error(),
        );
    }

    // Separate facet attributes from other attributes
    let (facet_attrs, non_facet_attrs): (Vec<_>, Vec<_>) = attrs
        .iter()
        .filter(|attr| !attr.path().is_ident(CODEGEN))
        .partition(|attr| attr.path().is_ident("facet"));

    let mut ffi_enum = input.clone();
    ffi_enum.ident = format_ident!("{}Ffi", enum_ident);
    ffi_enum.attrs = vec![];
    // `#[codegen(...)]` changes only the generated shell code, and rustc does
    // not know it, so no enum that this macro emits carries it.
    for variant in &mut ffi_enum.variants {
        variant.attrs.retain(|attr| !attr.path().is_ident(CODEGEN));
    }
    let ffi_enum_ident = &ffi_enum.ident;

    let ffi_enum = match typegen_kind {
        TypegenKind::Facet => {
            let facet_meta_attrs = facet_attrs.iter().map(|attr| &attr.meta);

            quote! {
                #[derive(::serde::Serialize, ::serde::Deserialize)]
                #[serde(rename = #enum_ident_str)]
                #[cfg_attr(
                    feature = "facet_typegen",
                    derive(::facet::Facet),
                    #(#facet_meta_attrs,)*
                    facet(rename = #enum_ident_str),
                    repr(C)
                )]
                #ffi_enum
            }
        }
        TypegenKind::None => quote! {},
    };

    // The bridge packs the variant index into 8 bits of the request id, so an
    // effect enum can have at most 256 variants.
    assert!(
        input.variants.len() <= MAX_VARIANTS,
        "an effect enum can have at most {MAX_VARIANTS} variants, because the bridge packs the \
         variant index into 8 bits of the request id, but `{enum_ident}` has {}",
        input.variants.len()
    );

    let variant_count = u16::try_from(input.variants.len()).expect("variant count checked above");

    let effects: Vec<Effect> = input
        .variants
        .into_iter()
        .map(|variant| {
            let group = variant_group(&variant.attrs).unwrap_or_else(|error| {
                errors.push(error.to_compile_error());
                None
            });
            let ident = variant.ident;
            let operation = variant
                .fields
                .into_iter()
                .next()
                .expect("each variant is expected to be a tuple with one field")
                .ty;
            Effect {
                ident,
                operation,
                group,
            }
        })
        .collect();
    let effects = effects.into_iter();

    let effect_names = effects
        .clone()
        .map(|effect| {
            let ident = effect.ident;
            let ident_str = ident.to_string();
            (ident, ident_str)
        })
        .collect::<Vec<_>>();

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
        let variant_name_arms = effect_names.iter().map(|(effect_ident, effect_ident_str)| {
            quote! {
                #enum_ident::#effect_ident(_) => #effect_ident_str
            }
        });
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
            }

            ::crux_core::__crux_core_testing_items! {
                impl #enum_ident {
                    #[doc(hidden)]
                    #[track_caller]
                    pub fn #expect_fn(self) -> ::crux_core::Request<#operation> {
                        match self {
                            #enum_ident::#effect_ident(request) => request,
                            effect => {
                                let actual = match &effect {
                                    #(#variant_name_arms,)*
                                };

                                panic!("expected {} effect, got {actual}", #effect_ident_str)
                            }
                        }
                    }
                }
            }
        }
    });

    let test_ext_trait_ident = format_ident!("{}TestExt", enum_ident);

    let test_ext_trait_methods = effects.clone().map(|effect| {
        let effect_ident_str = effect.ident.to_string();
        let effect_ident_snake = effect_ident_str.to_snake_case();
        let operation = &effect.operation;
        let expect_fn = Ident::new(&format!("expect_{effect_ident_snake}"), Span::call_site());
        let expect_with_fn = Ident::new(
            &format!("expect_{effect_ident_snake}_with"),
            Span::call_site(),
        );
        let expect_only_fn = Ident::new(
            &format!("expect_only_{effect_ident_snake}"),
            Span::call_site(),
        );
        let expect_only_with_fn = Ident::new(
            &format!("expect_only_{effect_ident_snake}_with"),
            Span::call_site(),
        );
        let resolve_fn = Ident::new(&format!("resolve_{effect_ident_snake}"), Span::call_site());
        quote! {
            fn #expect_fn(&mut self) -> &mut Self;
            fn #expect_with_fn<F>(&mut self, f: F) -> &mut Self
            where
                F: ::core::ops::FnOnce(&#operation);
            fn #expect_only_fn(&mut self);
            fn #expect_only_with_fn<F>(&mut self, f: F)
            where
                F: ::core::ops::FnOnce(&#operation);
            fn #resolve_fn<F>(&mut self, f: F) -> &mut Self
            where
                F: ::core::ops::FnOnce(&#operation)
                    -> <#operation as ::crux_core::capability::Operation>::Output;
        }
    });

    let test_ext_impl_methods = effects.clone().map(|effect| {
        let effect_ident_str = effect.ident.to_string();
        let effect_ident_snake = effect_ident_str.to_snake_case();
        let operation = &effect.operation;
        let expect_fn = Ident::new(&format!("expect_{effect_ident_snake}"), Span::call_site());
        let expect_with_fn = Ident::new(
            &format!("expect_{effect_ident_snake}_with"),
            Span::call_site(),
        );
        let expect_only_fn = Ident::new(
            &format!("expect_only_{effect_ident_snake}"),
            Span::call_site(),
        );
        let expect_only_with_fn = Ident::new(
            &format!("expect_only_{effect_ident_snake}_with"),
            Span::call_site(),
        );
        let resolve_fn = Ident::new(&format!("resolve_{effect_ident_snake}"), Span::call_site());
        let no_more_msg = format!("expected {effect_ident_str} effect but no more effects remain");
        quote! {
            #[track_caller]
            fn #expect_fn(&mut self) -> &mut Self {
                let effect = self.effects().next()
                    .unwrap_or_else(|| panic!(#no_more_msg));
                let _ = effect.#expect_fn();
                self
            }

            #[track_caller]
            fn #expect_with_fn<F>(&mut self, f: F) -> &mut Self
            where
                F: ::core::ops::FnOnce(&#operation),
            {
                let effect = self.effects().next()
                    .unwrap_or_else(|| panic!(#no_more_msg));
                let req = effect.#expect_fn();
                f(&req.operation);
                self
            }

            #[track_caller]
            fn #expect_only_fn(&mut self) {
                let effect = self.effects().next()
                    .unwrap_or_else(|| panic!(#no_more_msg));
                let _ = effect.#expect_fn();
                let remaining_effects = self.effects().count();
                let remaining_events = self.events().count();
                assert!(
                    remaining_effects + remaining_events == 0,
                    "expected command to be done, found {remaining_effects} effects and {remaining_events} events",
                );
            }

            #[track_caller]
            fn #expect_only_with_fn<F>(&mut self, f: F)
            where
                F: ::core::ops::FnOnce(&#operation),
            {
                let effect = self.effects().next()
                    .unwrap_or_else(|| panic!(#no_more_msg));
                let req = effect.#expect_fn();
                f(&req.operation);
                let remaining_effects = self.effects().count();
                let remaining_events = self.events().count();
                assert!(
                    remaining_effects + remaining_events == 0,
                    "expected command to be done, found {remaining_effects} effects and {remaining_events} events",
                );
            }

            #[track_caller]
            fn #resolve_fn<F>(&mut self, f: F) -> &mut Self
            where
                F: ::core::ops::FnOnce(&#operation)
                    -> <#operation as ::crux_core::capability::Operation>::Output,
            {
                let effect = self.effects().next()
                    .unwrap_or_else(|| panic!(#no_more_msg));
                let mut req = effect.#expect_fn();
                let output = f(&req.operation);
                req.resolve(output).expect("resolve failed");
                self
            }
        }
    });

    let test_ext = quote! {
        ::crux_core::__crux_core_testing_items! {
            #[doc(hidden)]
            pub trait #test_ext_trait_ident<Event>
            where
                Event: ::core::marker::Send + 'static,
            {
                #(#test_ext_trait_methods)*

                fn then_event<F>(&mut self, f: F) -> &mut Self
                where
                    F: ::core::ops::FnOnce(&Event);
            }

            #[doc(hidden)]
            impl<Event> #test_ext_trait_ident<Event> for ::crux_core::Command<#enum_ident, Event>
            where
                Event: ::core::marker::Send + 'static,
            {
                #(#test_ext_impl_methods)*

                #[track_caller]
                fn then_event<F>(&mut self, f: F) -> &mut Self
                where
                    F: ::core::ops::FnOnce(&Event),
                {
                    let ev = self.events().next()
                        .unwrap_or_else(|| panic!("expected an event but got none"));
                    f(&ev);
                    self
                }
            }
        }
    };

    let type_gen = match typegen_kind {
        TypegenKind::Facet => {
            let effect_gen = effects.clone().map(|effect| {
                let operation = &effect.operation;

                quote! {
                    let generator = <#operation>::register_types_facet(generator)
                        .map_err(|err| ::crux_core::type_generation::facet::TypeGenError::Generation(err.to_string()))?;
                }
            });
            let effect_variants_meta = effects.map(|effect| {
                let ident_str = effect.ident.to_string();
                let operation = &effect.operation;
                let group = effect.group.as_ref().map(|group| quote! { .group(#group) });

                quote! {
                    .variant::<#operation>(#ident_str)? #group
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
                            .map_err(|err| ::crux_core::type_generation::facet::TypeGenError::Generation(err.to_string()))?
                            .register_type::<::crux_core::bridge::Requests<#ffi_enum_ident>>()
                            .map_err(|err| ::crux_core::type_generation::facet::TypeGenError::Generation(err.to_string()))?;

                        generator
                            .register_effect::<#ffi_enum_ident>()?
                            #(#effect_variants_meta)*
                            .finish();

                        Ok(generator)
                    }
                }
            }
        }
        TypegenKind::None => quote! {},
    };

    let variant_index_arms = effect_names.iter().enumerate().map(|(index, (ident, _))| {
        let index = u8::try_from(index).expect("variant count checked above");
        quote! {
            #enum_ident::#ident(_) => #index
        }
    });

    // The reverse of `variant_index`: a request id carries only the index, so
    // this is what lets an error about one name the effect it claims.
    let variant_name_by_index_arms = effect_names.iter().enumerate().map(|(index, (_, name))| {
        let index = u8::try_from(index).expect("variant count checked above");
        quote! {
            #index => Some(#name)
        }
    });

    let effect_ffi_derive = if matches!(typegen_kind, TypegenKind::None) {
        quote! {}
    } else {
        quote! {
            impl crux_core::EffectFFI for #enum_ident {
                type Ffi = #ffi_enum_ident;
                const VARIANT_COUNT: u16 = #variant_count;
                fn variant_index(&self) -> u8 {
                    match self {
                        #(#variant_index_arms ,)*
                    }
                }
                fn variant_name(index: u8) -> Option<&'static str> {
                    match index {
                        #(#variant_name_by_index_arms ,)*
                        _ => None,
                    }
                }
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
        #(#errors)*

        #original_enum

        #ffi_enum

        impl crux_core::Effect for #enum_ident {}

        #effect_ffi_derive

        #(#from_impls)*

        #(#filters)*

        #test_ext

        #type_gen

    }
}
