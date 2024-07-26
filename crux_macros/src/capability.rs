use darling::{ast, util, FromDeriveInput, FromField, ToTokens};
use proc_macro2::TokenStream;
use proc_macro_error::{abort, OptionExt};
use quote::quote;
use syn::{DeriveInput, GenericArgument, Ident, PathArguments, Type};

#[derive(FromDeriveInput, Debug)]
#[darling(supports(struct_named))]
struct CapabilityStructReceiver {
    ident: Ident,
    data: ast::Data<util::Ignored, CapabilityFieldReceiver>,
}

#[derive(FromField, Debug)]
pub struct CapabilityFieldReceiver {
    ident: Option<Ident>,
    ty: Type,
}

impl ToTokens for CapabilityStructReceiver {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = &self.ident;
        let operation_type = self
            .data
            .as_ref()
            .take_struct()
            .expect_or_abort("should be a struct")
            .fields
            .iter()
            .find(|f| f.ident.as_ref().unwrap() == "context")
            .map(|f| first_generic_parameter(&f.ty))
            .expect_or_abort("could not find a field named `context`");

        tokens.extend(quote! {
          impl<Ev> crux_core::capability::Capability<Ev> for #name<Ev> {
            type Operation = #operation_type;
            type MappedSelf<MappedEv> = #name<MappedEv>;

            fn map_event<F, NewEv>(&self, f: F) -> Self::MappedSelf<NewEv>
            where
                F: Fn(NewEv) -> Ev + Send + Sync + 'static,
                Ev: 'static,
                NewEv: 'static + Send,
            {
              #name::new(self.context.map_event(f))
            }
          }
        })
    }
}

pub(crate) fn capability_impl(input: &DeriveInput) -> TokenStream {
    let input = match CapabilityStructReceiver::from_derive_input(input) {
        Ok(v) => v,
        Err(e) => {
            return e.write_errors();
        }
    };

    quote!(#input)
}

fn first_generic_parameter(ty: &Type) -> Type {
    let generic_param = match ty.clone() {
        Type::Path(mut path) if path.qself.is_none() => {
            // Get the last segment of the path where the generic parameters should be
            let last = path.path.segments.last_mut().expect("type has no segments");
            let type_params = std::mem::take(&mut last.arguments);

            let first_type_parameter = match type_params {
                PathArguments::AngleBracketed(params) => params.args.first().cloned(),
                _ => None,
            };

            // This argument must be a type
            match first_type_parameter {
                Some(GenericArgument::Type(t2)) => Some(t2),
                _ => None,
            }
        }
        _ => None,
    };
    let Some(generic_param) = generic_param else {
        abort!(ty, "context field type should have generic type parameters");
    };
    generic_param
}

#[cfg(test)]
mod tests {
    use darling::{FromDeriveInput, FromMeta};
    use quote::quote;
    use syn::{parse_str, Type};

    use crate::capability::CapabilityStructReceiver;

    use super::first_generic_parameter;

    #[test]
    fn test_derive() {
        let input = r#"
            #[derive(Capability)]
            pub struct Render<Ev> {
              context: CapabilityContext<RenderOperation, Ev>,
            }
        "#;
        let input = parse_str(input).unwrap();
        let input = CapabilityStructReceiver::from_derive_input(&input).unwrap();

        let actual = quote!(#input);

        insta::assert_snapshot!(pretty_print(&actual), @r###"
        impl<Ev> crux_core::capability::Capability<Ev> for Render<Ev> {
            type Operation = RenderOperation;
            type MappedSelf<MappedEv> = Render<MappedEv>;
            fn map_event<F, NewEv>(&self, f: F) -> Self::MappedSelf<NewEv>
            where
                F: Fn(NewEv) -> Ev + Send + Sync + 'static,
                Ev: 'static,
                NewEv: 'static + Send,
            {
                Render::new(self.context.map_event(f))
            }
        }
        "###);
    }

    #[test]
    fn test_first_generic_parameter() {
        let ty = Type::from_string("CapabilityContext<my_mod::MyOperation, Ev>").unwrap();

        let first_param = first_generic_parameter(&ty);

        assert_eq!(
            quote!(#first_param).to_string(),
            quote!(my_mod::MyOperation).to_string()
        );
    }

    fn pretty_print(ts: &proc_macro2::TokenStream) -> String {
        let file = syn::parse_file(&ts.to_string()).unwrap();
        prettyplease::unparse(&file)
    }
}
