use darling::{ast, util, FromDeriveInput, FromField, ToTokens};
use proc_macro2::TokenStream;
use proc_macro_error::OptionExt;
use quote::{format_ident, quote};
use syn::{DeriveInput, Ident, Type};

#[derive(FromDeriveInput, Debug)]
#[darling(attributes(effect), supports(struct_named))]
struct ExportStructReceiver {
    ident: Ident,
    name: Option<Ident>, // also used by the effect derive macro to name the effect
    data: ast::Data<util::Ignored, ExportFieldReceiver>,
}

#[derive(FromField, Debug)]
#[darling(attributes(effect))]
pub struct ExportFieldReceiver {
    ty: Type,
    #[darling(default)]
    skip: bool,
}

impl ToTokens for ExportStructReceiver {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ident = &self.ident;

        let ffi_export_name = match self.name {
            Some(ref name) => {
                let ffi_ef_name = format_ident!("{}Ffi", name);

                quote!(#ffi_ef_name)
            }
            None => quote!(EffectFfi),
        };

        let fields: Vec<&ExportFieldReceiver> = self
            .data
            .as_ref()
            .take_struct()
            .expect_or_abort("should be a struct")
            .fields
            .into_iter()
            .filter(|e| !e.skip)
            .collect();

        let mut output_type_exports = Vec::new();

        for capability in fields.iter().map(|f| &f.ty) {
            output_type_exports.push(quote! {
                #capability::register_types(generator)?;
            });
        }

        tokens.extend(quote! {

            impl ::crux_core::typegen::Export for #ident {
                #[cfg(feature = "typegen")]
                fn register_types(generator: &mut ::crux_core::typegen::TypeGen) -> ::crux_core::typegen::Result {
                    use ::crux_core::capability::Capability;
                    #(#output_type_exports)*
                    generator.register_type::<#ffi_export_name>()?;
                    generator.register_type::<::crux_core::bridge::Request<#ffi_export_name>>()?;

                    Ok(())
                }
            }
        })
    }
}

pub(crate) fn export_impl(input: &DeriveInput) -> TokenStream {
    let input = match ExportStructReceiver::from_derive_input(input) {
        Ok(v) => v,
        Err(e) => {
            return e.write_errors();
        }
    };

    quote!(#input)
}

#[cfg(test)]
mod tests {
    use darling::FromDeriveInput;
    use quote::quote;
    use syn::parse_str;

    use crate::export::ExportStructReceiver;

    #[test]
    fn defaults() {
        let input = r#"
            #[derive(Export)]
            pub struct Capabilities {
                pub render: Render,
            }
        "#;
        let input = parse_str(input).unwrap();
        let input = ExportStructReceiver::from_derive_input(&input).unwrap();

        let actual = quote!(#input);

        insta::assert_snapshot!(pretty_print(&actual), @r###"
        impl ::crux_core::typegen::Export for Capabilities {
            #[cfg(feature = "typegen")]
            fn register_types(
                generator: &mut ::crux_core::typegen::TypeGen,
            ) -> ::crux_core::typegen::Result {
                use ::crux_core::capability::Capability;
                Render::register_types(generator)?;
                generator.register_type::<EffectFfi>()?;
                generator.register_type::<::crux_core::bridge::Request<EffectFfi>>()?;
                Ok(())
            }
        }
        "###);
    }

    #[test]
    fn export_macro_respects_an_skip_attr() {
        let input = r#"
            #[derive(Export)]
            pub struct MyCapabilities {
                pub http: crux_http::Http,
                pub key_value: KeyValue,
                pub platform: Platform,
                pub render: Render,
                #[effect(skip)]
                pub time: Time,
            }
        "#;
        let input = parse_str(input).unwrap();
        let input = ExportStructReceiver::from_derive_input(&input).unwrap();

        let actual = quote!(#input);

        insta::assert_snapshot!(pretty_print(&actual), @r###"
        impl ::crux_core::typegen::Export for MyCapabilities {
            #[cfg(feature = "typegen")]
            fn register_types(
                generator: &mut ::crux_core::typegen::TypeGen,
            ) -> ::crux_core::typegen::Result {
                use ::crux_core::capability::Capability;
                crux_http::Http::register_types(generator)?;
                KeyValue::register_types(generator)?;
                Platform::register_types(generator)?;
                Render::register_types(generator)?;
                generator.register_type::<EffectFfi>()?;
                generator.register_type::<::crux_core::bridge::Request<EffectFfi>>()?;
                Ok(())
            }
        }
        "###);
    }

    #[test]
    fn full() {
        let input = r#"
            #[derive(Export)]
            pub struct MyCapabilities {
                pub http: crux_http::Http,
                pub key_value: KeyValue,
                pub platform: Platform,
                pub render: Render,
                pub time: Time,
            }
        "#;
        let input = parse_str(input).unwrap();
        let input = ExportStructReceiver::from_derive_input(&input).unwrap();

        let actual = quote!(#input);

        insta::assert_snapshot!(pretty_print(&actual), @r###"
        impl ::crux_core::typegen::Export for MyCapabilities {
            #[cfg(feature = "typegen")]
            fn register_types(
                generator: &mut ::crux_core::typegen::TypeGen,
            ) -> ::crux_core::typegen::Result {
                use ::crux_core::capability::Capability;
                crux_http::Http::register_types(generator)?;
                KeyValue::register_types(generator)?;
                Platform::register_types(generator)?;
                Render::register_types(generator)?;
                Time::register_types(generator)?;
                generator.register_type::<EffectFfi>()?;
                generator.register_type::<::crux_core::bridge::Request<EffectFfi>>()?;
                Ok(())
            }
        }
        "###);
    }

    #[test]
    fn export_macro_respects_an_effect_name_override() {
        let input = r#"
            #[derive(Export, Effect)]
            #[effect(name = "MyEffect")]
            pub struct Capabilities {
                render: Render,
            }
        "#;

        let input = parse_str(input).unwrap();
        let input = ExportStructReceiver::from_derive_input(&input).unwrap();

        let actual = quote!(#input);

        insta::assert_snapshot!(pretty_print(&actual), @r###"
        impl ::crux_core::typegen::Export for Capabilities {
            #[cfg(feature = "typegen")]
            fn register_types(
                generator: &mut ::crux_core::typegen::TypeGen,
            ) -> ::crux_core::typegen::Result {
                use ::crux_core::capability::Capability;
                Render::register_types(generator)?;
                generator.register_type::<MyEffectFfi>()?;
                generator.register_type::<::crux_core::bridge::Request<MyEffectFfi>>()?;
                Ok(())
            }
        }
        "###);
    }

    fn pretty_print(ts: &proc_macro2::TokenStream) -> String {
        let file = syn::parse_file(&ts.to_string()).unwrap();
        prettyplease::unparse(&file)
    }
}
