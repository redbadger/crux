use syn::parse_quote;

use crate::pretty_print;

use super::macro_impl::operation_impl;

#[test]
fn notify_unit_struct() {
    let input = parse_quote! {
        #[operation(notify)]
        pub struct Subscribe;
    };

    let actual = operation_impl(&input);

    insta::assert_snapshot!(pretty_print(&actual), @r"
    impl ::crux_core::capability::Operation for Subscribe {
        type Output = ();
        const KIND: ::core::option::Option<::crux_core::RequestKind> = ::core::option::Option::Some(
            ::crux_core::RequestKind::Notify,
        );
    }
    impl ::crux_core::operation::Notify for Subscribe {}
    ");
}

#[test]
fn request_named_struct_with_generic_output_type() {
    let input = parse_quote! {
        #[operation(request, output = Option<Vec<u8>>)]
        pub struct Get {
            pub key: String,
        }
    };

    let actual = operation_impl(&input);

    insta::assert_snapshot!(pretty_print(&actual), @"
    impl ::crux_core::capability::Operation for Get {
        type Output = Option<Vec<u8>>;
        const KIND: ::core::option::Option<::crux_core::RequestKind> = ::core::option::Option::Some(
            ::crux_core::RequestKind::Request,
        );
    }
    impl ::crux_core::operation::Request for Get {}
    ");
}

#[test]
fn stream_tuple_struct() {
    let input = parse_quote! {
        #[operation(stream, output = Message)]
        pub struct Listen(pub String);
    };

    let actual = operation_impl(&input);

    insta::assert_snapshot!(pretty_print(&actual), @r"
    impl ::crux_core::capability::Operation for Listen {
        type Output = Message;
        const KIND: ::core::option::Option<::crux_core::RequestKind> = ::core::option::Option::Some(
            ::crux_core::RequestKind::Stream,
        );
    }
    impl ::crux_core::operation::Stream for Listen {}
    ");
}

#[test]
fn register_extra_types() {
    let input = parse_quote! {
        #[operation(request, output = ValueResult, register(KeyValueError, Value))]
        pub struct Get {
            pub key: String,
        }
    };

    let actual = operation_impl(&input);

    insta::assert_snapshot!(pretty_print(&actual), @r#"
    #[allow(unexpected_cfgs)]
    impl ::crux_core::capability::Operation for Get {
        type Output = ValueResult;
        const KIND: ::core::option::Option<::crux_core::RequestKind> = ::core::option::Option::Some(
            ::crux_core::RequestKind::Request,
        );
        #[cfg(feature = "typegen")]
        fn register_types(
            generator: &mut ::crux_core::type_generation::serde::TypeGen,
        ) -> ::crux_core::type_generation::serde::Result
        where
            Self: ::serde::Serialize + for<'de> ::serde::de::Deserialize<'de>,
            <Self as ::crux_core::capability::Operation>::Output: for<'de> ::serde::de::Deserialize<
                'de,
            >,
        {
            generator.register_type::<KeyValueError>()?;
            generator.register_type::<Value>()?;
            generator.register_type::<Self>()?;
            generator
                .register_type::<<Self as ::crux_core::capability::Operation>::Output>()?;
            Ok(())
        }
        #[cfg(feature = "facet_typegen")]
        fn register_types_facet<'facet>(
            generator: &mut ::crux_core::type_generation::facet::TypeRegistry,
        ) -> ::core::result::Result<
            &mut ::crux_core::type_generation::facet::TypeRegistry,
            ::crux_core::type_generation::facet::TypeGenError,
        >
        where
            Self: ::facet::Facet<'facet> + ::serde::Serialize
                + for<'de> ::serde::de::Deserialize<'de>,
            <Self as ::crux_core::capability::Operation>::Output: ::facet::Facet<'facet>
                + for<'de> ::serde::de::Deserialize<'de>,
        {
            generator
                .register_type::<KeyValueError>()
                .map_err(|e| ::crux_core::type_generation::facet::TypeGenError::Generation(
                    e.to_string(),
                ))?
                .register_type::<Value>()
                .map_err(|e| ::crux_core::type_generation::facet::TypeGenError::Generation(
                    e.to_string(),
                ))?
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
    }
    impl ::crux_core::operation::Request for Get {}
    "#);
}

#[test]
fn generic_struct_with_where_clause() {
    let input = parse_quote! {
        #[operation(request, output = Vec<T>)]
        pub struct Fetch<T>
        where
            T: Send + 'static,
        {
            pub seed: T,
        }
    };

    let actual = operation_impl(&input);

    insta::assert_snapshot!(pretty_print(&actual), @"
    impl<T> ::crux_core::capability::Operation for Fetch<T>
    where
        T: Send + 'static,
    {
        type Output = Vec<T>;
        const KIND: ::core::option::Option<::crux_core::RequestKind> = ::core::option::Option::Some(
            ::crux_core::RequestKind::Request,
        );
    }
    impl<T> ::crux_core::operation::Request for Fetch<T>
    where
        T: Send + 'static,
    {}
    ");
}

#[test]
fn no_kind_declared() {
    let input = parse_quote! {
        pub struct Get {
            pub key: String,
        }
    };

    insta::assert_snapshot!(pretty_print(&operation_impl(&input)), @r#"
    ::core::compile_error! {
        "an operation must declare its request kind: add `#[operation(notify)]`, `#[operation(request, output = T)]` or `#[operation(stream, output = T)]`"
    }
    "#);
}

#[test]
fn two_kinds_declared() {
    let input = parse_quote! {
        #[operation(notify, request, output = u8)]
        pub struct Get;
    };

    insta::assert_snapshot!(pretty_print(&operation_impl(&input)), @r#"
    ::core::compile_error! {
        "an operation declares exactly one request kind, but `notify`, `request` were all given"
    }
    "#);
}

#[test]
fn notify_cannot_have_an_output() {
    let input = parse_quote! {
        #[operation(notify, output = u8)]
        pub struct Publish;
    };

    insta::assert_snapshot!(pretty_print(&operation_impl(&input)), @r#"
    ::core::compile_error! {
        "a `notify` operation is never answered, so it cannot declare an `output`; its `Operation::Output` is `()`"
    }
    "#);
}

#[test]
fn request_needs_an_output() {
    let input = parse_quote! {
        #[operation(request)]
        pub struct Get;
    };

    insta::assert_snapshot!(pretty_print(&operation_impl(&input)), @r#"
    ::core::compile_error! {
        "a `request` operation is answered with an `Operation::Output`, so it must declare one: `#[operation(request, output = T)]`"
    }
    "#);
}

#[test]
fn misspelled_kind_suggests_the_right_word() {
    let input = parse_quote! {
        #[operation(reqest, output = u8)]
        pub struct Get;
    };

    let actual = operation_impl(&input).to_string();

    assert!(
        actual.contains("Did you mean `request`"),
        "expected a suggestion, got: {actual}"
    );
}

#[test]
fn enums_are_not_operations() {
    let input = parse_quote! {
        #[operation(notify)]
        pub enum Get {
            One,
        }
    };

    let actual = operation_impl(&input).to_string();

    assert!(
        actual.contains("Unsupported shape"),
        "expected a shape error, got: {actual}"
    );
}
