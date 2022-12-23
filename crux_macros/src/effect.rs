use darling::{
    ast::{Data, Fields},
    FromField, FromMeta,
};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{AttributeArgs, ItemStruct, Type};

#[derive(Debug, FromMeta)]
struct EffectArgs {
    #[darling(default)]
    name: Option<Type>,
}

#[derive(Debug, FromField)]
#[darling(attributes(operation))]
struct FieldArgs {
    #[darling(default)]
    ty: Type,
}

pub(crate) fn effect_impl(attr_args: AttributeArgs, item: ItemStruct) -> TokenStream {
    let args = match EffectArgs::from_list(&attr_args) {
        Ok(v) => v,
        Err(e) => {
            return e.write_errors();
        }
    };
    let ident = &item.ident;
    let name = match args.name {
        Some(name) => name,
        None => Type::from_string("Effect").unwrap(),
    };

    // if let Data::Struct(s) = &item.data {
    //     if let Fields::Named(fields) = &s.fields {
    //         for f in &fields.named {
    //             let props = FieldProps::from_field(&f);

    //             println!("{}: {:?}", f.ident.as_ref().unwrap().to_string(), props);
    //         }
    //     }
    // }
    quote! {
        #item

        #[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
        pub enum #name {
            Http(HttpRequest),
            Render,
        }

        impl crux_core::WithContext<App, #name> for #ident {
            fn new_with_context(context: CapabilityContext<#name, Event>) -> #ident {
                #ident {
                    http: Http::new(context.with_effect(#name::Http)),
                    render: Render::new(context.with_effect(|_| #name::Render)),
                }
            }
        }
    }
}
