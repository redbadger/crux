// darling's generated `FromDeriveInput` implementation trips this nursery lint,
// and the diagnostic is spanned onto our `#[darling(default)]` attributes.
#![allow(clippy::option_if_let_else)]

use darling::{FromDeriveInput, FromMeta, ToTokens, ast::MetaNameValueInvalidExpr, util::PathList};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Generics, Ident, Meta, Type};

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

/// The three request kinds an operation can declare.
#[derive(Clone, Copy)]
enum Kind {
    Notify,
    Request,
    Stream,
}

impl Kind {
    /// The `RequestKind` variant and the marker trait share a name.
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
    /// Extra types to register with type generation, e.g. `register(Error, Value)`.
    #[darling(default)]
    register: PathList,
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
                "an operation must declare its request kind: add `#[operation(notify)]`, \
                 `#[operation(request, output = T)]` or `#[operation(stream, output = T)]`",
            )
            .with_span(&self.ident)),
            many => {
                let words: Vec<&str> = many.iter().map(|kind| kind.word()).collect();
                Err(darling::Error::custom(format!(
                    "an operation declares exactly one request kind, but `{}` were all given",
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
        let typegen = self.typegen();
        let allow_unexpected_cfgs = self.allow_unexpected_cfgs();

        Ok(quote! {
            #allow_unexpected_cfgs
            impl #impl_generics ::crux_core::capability::Operation for #ident #ty_generics #where_clause {
                type Output = #output;

                const KIND: ::core::option::Option<::crux_core::RequestKind> =
                    ::core::option::Option::Some(::crux_core::RequestKind::#kind_ident);

                #typegen
            }

            impl #impl_generics ::crux_core::operation::#kind_ident for #ident #ty_generics #where_clause {}
        })
    }

    /// The type generation hooks are gated on the `typegen` and
    /// `facet_typegen` features of the crate the derive is used in, and most
    /// crates that declare operations declare neither. An undeclared feature in
    /// a `cfg` is an `unexpected_cfgs` warning — fatal under `-D warnings` —
    /// so the generated `impl` carries an `allow`. A lint attribute on the
    /// `fn` itself is too late: the item is stripped before the lint is
    /// levelled, and only the enclosing item's level is consulted.
    fn allow_unexpected_cfgs(&self) -> TokenStream {
        if self.register.is_empty() {
            return quote! {};
        }

        quote! { #[allow(unexpected_cfgs)] }
    }

    /// The type generation hooks, emitted only when `register(..)` names extra
    /// types. Both are gated on features of the crate the derive is used in.
    fn typegen(&self) -> TokenStream {
        if self.register.is_empty() {
            return quote! {};
        }

        let register = self.register.iter();
        let serde = quote! {
            #[cfg(feature = "typegen")]
            fn register_types(
                generator: &mut ::crux_core::type_generation::serde::TypeGen,
            ) -> ::crux_core::type_generation::serde::Result
            where
                Self: ::serde::Serialize + for<'de> ::serde::de::Deserialize<'de>,
                <Self as ::crux_core::capability::Operation>::Output:
                    for<'de> ::serde::de::Deserialize<'de>,
            {
                #(generator.register_type::<#register>()?;)*
                generator.register_type::<Self>()?;
                generator.register_type::<<Self as ::crux_core::capability::Operation>::Output>()?;
                Ok(())
            }
        };

        let register = self.register.iter();
        let facet = quote! {
            #[cfg(feature = "facet_typegen")]
            fn register_types_facet<'facet>(
                generator: &mut ::crux_core::type_generation::facet::TypeRegistry,
            ) -> ::core::result::Result<
                &mut ::crux_core::type_generation::facet::TypeRegistry,
                ::crux_core::type_generation::facet::TypeGenError,
            >
            where
                Self: ::facet::Facet<'facet> + ::serde::Serialize + for<'de> ::serde::de::Deserialize<'de>,
                <Self as ::crux_core::capability::Operation>::Output:
                    ::facet::Facet<'facet> + for<'de> ::serde::de::Deserialize<'de>,
            {
                generator
                    #(.register_type::<#register>()
                        .map_err(|e| ::crux_core::type_generation::facet::TypeGenError::Generation(
                            e.to_string(),
                        ))?)*
                    .register_type::<Self>()
                    .map_err(|e| ::crux_core::type_generation::facet::TypeGenError::Generation(
                        e.to_string(),
                    ))?
                    .register_type::<<Self as ::crux_core::capability::Operation>::Output>()
                    .map_err(|e| ::crux_core::type_generation::facet::TypeGenError::Generation(
                        e.to_string(),
                    ))?;

                Ok(generator)
            }
        };

        quote! {
            #serde
            #facet
        }
    }
}
