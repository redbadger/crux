use proc_macro2::TokenStream;
use quote::quote;
use syn::{Type, TypeTuple};

pub fn caps_macro_impl(input: TypeTuple) -> TokenStream {
    let types: Vec<Type> = input.elems.into_iter().collect();
    let event = types[0].clone();
    let effect = types[1].clone();

    let caps: Vec<Type> = types[2..].to_vec();
    let registrations = caps.iter().map(|cap| {
        quote! {
            #cap::<#event>::register_types(generator)?;
        }
    });

    quote! {
        pub struct Capabilities {}

        impl ::crux_core::WithContext<#event, #effect> for Capabilities {
            fn new_with_context(_context: ::crux_core::capability::ProtoContext<#effect, #event>) -> Self {
                Capabilities {}
            }
        }

        #[cfg(feature = "typegen")]
        impl crux_core::typegen::Export for Capabilities {
            fn register_types(generator: &mut crux_core::typegen::TypeGen) -> crux_core::typegen::Result {
                use crux_core::Capability;
                #(#registrations ;)*
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
    fn simple() {
        let input = parse_quote!((Event, Effect, Render));

        let actual = caps_macro_impl(input);

        insta::assert_snapshot!(pretty_print(&actual), @r#"
        pub struct Capabilities {}
        impl ::crux_core::WithContext<Event, Effect> for Capabilities {
            fn new_with_context(
                _context: ::crux_core::capability::ProtoContext<Effect, Event>,
            ) -> Self {
                Capabilities {}
            }
        }
        #[cfg(feature = "typegen")]
        impl crux_core::typegen::Export for Capabilities {
            fn register_types(
                generator: &mut crux_core::typegen::TypeGen,
            ) -> crux_core::typegen::Result {
                use crux_core::Capability;
                Render::<Event>::register_types(generator)?;
                Ok(())
            }
        }
        "#);
    }

    fn pretty_print(ts: &proc_macro2::TokenStream) -> String {
        let file = syn::parse_file(&ts.to_string()).unwrap();
        prettyplease::unparse(&file)
    }
}
