use darling::{ast, util, FromDeriveInput, FromField, ToTokens};
use proc_macro2::TokenStream;
use proc_macro_error::OptionExt;
use quote::{format_ident, quote};
use syn::{DeriveInput, GenericArgument, Ident, PathArguments, Type};

#[derive(FromDeriveInput, Debug)]
#[darling(attributes(effect), supports(struct_named))]
struct ExportStructReceiver {
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
        let effect_name = self.name.clone().unwrap_or_else(|| format_ident!("Effect"));
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

        for (capability, event) in fields.iter().map(|f| split_on_generic(&f.ty)) {
            output_type_exports.push(quote! {
                <#capability::<#event> as Capability<#event>>::Operation::register_types(generator)?;
            });
        }

        tokens.extend(quote! {
            impl ::crux_core::typegen::Export for #effect_name {
                fn register_types(generator: &mut ::crux_core::typegen::TypeGen) -> ::crux_core::typegen::Result {
                    use ::crux_core::capability::{Capability, Operation};
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

fn split_on_generic(ty: &Type) -> (Type, Type) {
    let ty = ty.clone();
    match ty {
        Type::Path(mut path) if path.qself.is_none() => {
            // Get the last segment of the path where the generic parameter should be

            let last = path.path.segments.last_mut().expect("type has no segments");
            let type_params = std::mem::take(&mut last.arguments);

            // It should have only one angle-bracketed param
            let generic_arg = match type_params {
                PathArguments::AngleBracketed(params) => params.args.first().cloned(),
                _ => None,
            };

            // This argument must be a type
            match generic_arg {
                Some(GenericArgument::Type(t2)) => Some((Type::Path(path), t2)),
                _ => None,
            }
        }
        _ => None,
    }
    .expect_or_abort("capabilities should be generic over a single event type")
}

#[cfg(test)]
mod tests {
    use darling::{FromDeriveInput, FromMeta};
    use quote::quote;
    use syn::{parse_str, Type};

    use crate::export::ExportStructReceiver;

    use super::split_on_generic;

    #[test]
    fn defaults() {
        let input = r#"
            #[derive(Export)]
            pub struct Capabilities {
                pub render: Render<Event>,
            }
        "#;
        let input = parse_str(input).unwrap();
        let input = ExportStructReceiver::from_derive_input(&input).unwrap();

        let actual = quote!(#input);

        insta::assert_snapshot!(pretty_print(&actual), @r"
        impl ::crux_core::typegen::Export for Effect {
            fn register_types(
                generator: &mut ::crux_core::typegen::TypeGen,
            ) -> ::crux_core::typegen::Result {
                use ::crux_core::capability::{Capability, Operation};
                <Render<Event> as Capability<Event>>::Operation::register_types(generator)?;
                generator.register_type::<EffectFfi>()?;
                generator.register_type::<::crux_core::bridge::Request<EffectFfi>>()?;
                Ok(())
            }
        }
        ");
    }

    #[test]
    fn split_event_types_preserves_path() {
        let ty = Type::from_string("crux_core::render::Render<Event>").unwrap();

        let (actual_type, actual_event) = split_on_generic(&ty);

        assert_eq!(
            quote!(#actual_type).to_string(),
            quote!(crux_core::render::Render).to_string()
        );

        assert_eq!(quote!(#actual_event).to_string(), quote!(Event).to_string());
    }

    #[test]
    fn export_macro_respects_an_skip_attr() {
        let input = r#"
            #[derive(Export)]
            pub struct MyCapabilities {
                pub http: crux_http::Http<MyEvent>,
                pub key_value: KeyValue<MyEvent>,
                pub platform: Platform<MyEvent>,
                pub render: Render<MyEvent>,
                #[effect(skip)]
                pub time: Time<MyEvent>,
            }
        "#;
        let input = parse_str(input).unwrap();
        let input = ExportStructReceiver::from_derive_input(&input).unwrap();

        let actual = quote!(#input);

        insta::assert_snapshot!(pretty_print(&actual), @r"
        impl ::crux_core::typegen::Export for Effect {
            fn register_types(
                generator: &mut ::crux_core::typegen::TypeGen,
            ) -> ::crux_core::typegen::Result {
                use ::crux_core::capability::{Capability, Operation};
                <crux_http::Http<
                    MyEvent,
                > as Capability<MyEvent>>::Operation::register_types(generator)?;
                <KeyValue<
                    MyEvent,
                > as Capability<MyEvent>>::Operation::register_types(generator)?;
                <Platform<
                    MyEvent,
                > as Capability<MyEvent>>::Operation::register_types(generator)?;
                <Render<MyEvent> as Capability<MyEvent>>::Operation::register_types(generator)?;
                generator.register_type::<EffectFfi>()?;
                generator.register_type::<::crux_core::bridge::Request<EffectFfi>>()?;
                Ok(())
            }
        }
        ");
    }

    #[test]
    fn full() {
        let input = r#"
            #[derive(Export)]
            pub struct MyCapabilities {
                pub http: crux_http::Http<MyEvent>,
                pub key_value: KeyValue<MyEvent>,
                pub platform: Platform<MyEvent>,
                pub render: Render<MyEvent>,
                pub time: Time<MyEvent>,
            }
        "#;
        let input = parse_str(input).unwrap();
        let input = ExportStructReceiver::from_derive_input(&input).unwrap();

        let actual = quote!(#input);

        insta::assert_snapshot!(pretty_print(&actual), @r"
        impl ::crux_core::typegen::Export for Effect {
            fn register_types(
                generator: &mut ::crux_core::typegen::TypeGen,
            ) -> ::crux_core::typegen::Result {
                use ::crux_core::capability::{Capability, Operation};
                <crux_http::Http<
                    MyEvent,
                > as Capability<MyEvent>>::Operation::register_types(generator)?;
                <KeyValue<
                    MyEvent,
                > as Capability<MyEvent>>::Operation::register_types(generator)?;
                <Platform<
                    MyEvent,
                > as Capability<MyEvent>>::Operation::register_types(generator)?;
                <Render<MyEvent> as Capability<MyEvent>>::Operation::register_types(generator)?;
                <Time<MyEvent> as Capability<MyEvent>>::Operation::register_types(generator)?;
                generator.register_type::<EffectFfi>()?;
                generator.register_type::<::crux_core::bridge::Request<EffectFfi>>()?;
                Ok(())
            }
        }
        ");
    }

    #[test]
    fn export_macro_respects_an_effect_name_override() {
        let input = r#"
            #[derive(Export, Effect)]
            #[effect(name = "MyEffect")]
            pub struct Capabilities {
                render: Render<Event>,
            }
        "#;

        let input = parse_str(input).unwrap();
        let input = ExportStructReceiver::from_derive_input(&input).unwrap();

        let actual = quote!(#input);

        insta::assert_snapshot!(pretty_print(&actual), @r"
        impl ::crux_core::typegen::Export for MyEffect {
            fn register_types(
                generator: &mut ::crux_core::typegen::TypeGen,
            ) -> ::crux_core::typegen::Result {
                use ::crux_core::capability::{Capability, Operation};
                <Render<Event> as Capability<Event>>::Operation::register_types(generator)?;
                generator.register_type::<MyEffectFfi>()?;
                generator.register_type::<::crux_core::bridge::Request<MyEffectFfi>>()?;
                Ok(())
            }
        }
        ");
    }

    fn pretty_print(ts: &proc_macro2::TokenStream) -> String {
        let file = syn::parse_file(&ts.to_string()).unwrap();
        prettyplease::unparse(&file)
    }
}
