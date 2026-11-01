// darling's generated `FromDeriveInput` implementation trips this nursery lint,
// and the diagnostic is spanned onto our `#[darling(default)]` attributes.
#![allow(clippy::option_if_let_else)]

use darling::{FromDeriveInput, FromMeta, ToTokens, ast::MetaNameValueInvalidExpr};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Generics, Ident, Meta, Path, Type};

/// The `output = T` value, absent unless declared.
///
/// An unquoted generic type such as `Option<Vec<u8>>` is not a valid
/// expression, so darling reaches it through
/// [`FromMeta::from_invalid_expr`] — which `syn::Type` implements but
/// `Option<T>` does not forward. Hence this newtype rather than a plain
/// `Option<syn::Type>` field.
#[derive(Default)]
struct MaybeOutput(Option<Type>);

impl FromMeta for MaybeOutput {
    fn from_none() -> Option<Self> {
        Some(Self(None))
    }

    fn from_meta(item: &Meta) -> darling::Result<Self> {
        Type::from_meta(item).map(|ty| Self(Some(ty)))
    }

    fn from_invalid_expr(value: &MetaNameValueInvalidExpr) -> darling::Result<Self> {
        Type::from_invalid_expr(value).map(|ty| Self(Some(ty)))
    }
}

/// The three kinds an operation can declare.
#[derive(Clone, Copy)]
enum Kind {
    Notify,
    Request,
    Stream,
}

impl Kind {
    /// The `OperationKind` variant and the marker trait share a name.
    fn ident(self) -> Ident {
        let name = match self {
            Self::Notify => "Notify",
            Self::Request => "Request",
            Self::Stream => "Stream",
        };
        Ident::new(name, proc_macro2::Span::call_site())
    }

    const fn word(self) -> &'static str {
        match self {
            Self::Notify => "notify",
            Self::Request => "request",
            Self::Stream => "stream",
        }
    }
}

#[derive(FromDeriveInput)]
#[darling(attributes(operation), supports(struct_any))]
struct OperationReceiver {
    ident: Ident,
    generics: Generics,
    /// `#[operation(notify)]` — told to the shell, never answered.
    #[darling(default)]
    notify: bool,
    /// `#[operation(request, output = T)]` — answered exactly once.
    #[darling(default)]
    request: bool,
    /// `#[operation(stream, output = T)]` — answered many times.
    #[darling(default)]
    stream: bool,
    /// The `Operation::Output` type. Forbidden for `notify`, required otherwise.
    #[darling(default)]
    output: MaybeOutput,
    /// `protocol = path`: the `ShellProtocol` of the capability that the
    /// operation belongs to. Type generation emits it for every app that uses
    /// the operation.
    #[darling(default)]
    protocol: Option<Path>,
}

pub fn operation_impl(input: &DeriveInput) -> TokenStream {
    let receiver = match OperationReceiver::from_derive_input(input) {
        Ok(receiver) => receiver,
        Err(error) => return error.write_errors(),
    };

    match receiver.expand() {
        Ok(tokens) => tokens,
        Err(error) => error.write_errors(),
    }
}

impl OperationReceiver {
    fn kind(&self) -> Result<Kind, darling::Error> {
        let declared: Vec<Kind> = [
            (self.notify, Kind::Notify),
            (self.request, Kind::Request),
            (self.stream, Kind::Stream),
        ]
        .into_iter()
        .filter_map(|(set, kind)| set.then_some(kind))
        .collect();

        match declared.as_slice() {
            [kind] => Ok(*kind),
            [] => Err(darling::Error::custom(
                "an operation must declare its kind: add `#[operation(notify)]`, \
                 `#[operation(request, output = T)]` or `#[operation(stream, output = T)]`",
            )
            .with_span(&self.ident)),
            many => {
                let words: Vec<&str> = many.iter().map(|kind| kind.word()).collect();
                Err(darling::Error::custom(format!(
                    "an operation declares exactly one kind, but `{}` were all given",
                    words.join("`, `")
                ))
                .with_span(&self.ident))
            }
        }
    }

    fn output(&self, kind: Kind) -> Result<TokenStream, darling::Error> {
        match (kind, self.output.0.as_ref()) {
            (Kind::Notify, None) => Ok(quote! { () }),
            (Kind::Notify, Some(output)) => Err(darling::Error::custom(
                "a `notify` operation is never answered, so it cannot declare an `output`; \
                 its `Operation::Output` is `()`",
            )
            .with_span(output)),
            (_, Some(output)) => Ok(output.to_token_stream()),
            (kind, None) => Err(darling::Error::custom(format!(
                "a `{}` operation is answered with an `Operation::Output`, \
                 so it must declare one: `#[operation({}, output = T)]`",
                kind.word(),
                kind.word()
            ))
            .with_span(&self.ident)),
        }
    }

    fn expand(&self) -> Result<TokenStream, darling::Error> {
        let kind = self.kind()?;
        let output = self.output(kind)?;

        let ident = &self.ident;
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();
        let kind_ident = kind.ident();
        let register_types = self.protocol.as_ref().map(register_types_facet);

        Ok(quote! {
            impl #impl_generics ::crux_core::capability::Operation for #ident #ty_generics #where_clause {
                type Output = #output;

                const KIND: ::core::option::Option<::crux_core::OperationKind> =
                    ::core::option::Option::Some(::crux_core::OperationKind::#kind_ident);

                #register_types
            }

            impl #impl_generics ::crux_core::operation::#kind_ident for #ident #ty_generics #where_clause {}
        })
    }
}

/// The `register_types_facet` override for an operation that names its
/// capability's protocol. It registers the operation and its output, as the
/// default does, and then the protocol.
///
/// The override is inside `crux_core::__crux_core_facet_typegen_items!`, so it
/// exists only when the `facet_typegen` feature of `crux_core` is on. The
/// compiler evaluates that feature in `crux_core`, and not in the capability
/// crate, so an app that turns on `crux_core/facet_typegen` gets the protocol.
fn register_types_facet(protocol: &Path) -> TokenStream {
    quote! {
        ::crux_core::__crux_core_facet_typegen_items! {
            fn register_types_facet<'a>(
                generator: &mut ::crux_core::type_generation::facet::TypeRegistry,
            ) -> ::core::result::Result<
                &mut ::crux_core::type_generation::facet::TypeRegistry,
                ::crux_core::type_generation::facet::TypeGenError,
            >
            where
                Self: ::facet::Facet<'a> + ::serde::Serialize + for<'de> ::serde::de::Deserialize<'de>,
                <Self as ::crux_core::capability::Operation>::Output:
                    ::facet::Facet<'a> + for<'de> ::serde::de::Deserialize<'de>,
            {
                generator
                    .register_type::<Self>()?
                    .register_type::<<Self as ::crux_core::capability::Operation>::Output>()?
                    .shell_protocol(&#protocol)
            }
        }
    }
}
