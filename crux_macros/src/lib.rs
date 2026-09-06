#![deny(clippy::pedantic)]

mod capability;
mod effect;
mod export;
mod operation;

use capability::capability_impl;
use export::export_impl;
use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;
use syn::{Ident, ItemEnum, parse_macro_input};

/// Generates an effect type matching the enum definition provided,
/// whilst supplying all the necessary decorations and additional trait implementations.
///
/// Use `typegen` as an argument if you want to opt in to the built-in foreign type generation.
///
/// e.g.
/// ```rust
/// # use crux_core::{render::RenderOperation};
/// # use crux_core::macros::effect;
/// # use crux_http::protocol::HttpRequest;
/// # #[derive(Default)]
/// # struct MyApp;
/// # pub enum MyEvent {None}
/// # impl crux_core::App for MyApp {
/// #     type Event = MyEvent;
/// #     type Model = ();
/// #     type ViewModel = ();
/// #     type Effect = MyEffect;
/// #     fn update(
/// #         &self,
/// #         _event: Self::Event,
/// #         _model: &mut Self::Model,
/// #     ) -> crux_core::Command<MyEffect, MyEvent> {
/// #         unimplemented!()
/// #     }
/// #     fn view(&self, _model: &Self::Model) -> Self::ViewModel {
/// #         unimplemented!()
/// #     }
/// # }
/// #[effect(typegen)]
/// pub enum MyEffect {
///     Render(RenderOperation),
///     Http(HttpRequest),
/// }
/// ```
#[proc_macro_attribute]
pub fn effect(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as Option<Ident>);
    let input = parse_macro_input!(input as ItemEnum);
    effect::macro_impl::effect_impl(args, input).into()
}

/// Implements `crux_core::capability::Operation` for a struct, declaring what
/// the shell does with it.
///
/// Exactly one request kind is required:
///
/// * `#[operation(notify)]` — the shell is told, and never answers.
///   `Operation::Output` is `()`, and the type also implements
///   `crux_core::operation::Notify`.
/// * `#[operation(request, output = T)]` — the shell answers exactly once with
///   a `T`. Also implements `crux_core::operation::Request`.
/// * `#[operation(stream, output = T)]` — the shell answers a sequence of `T`s.
///   Also implements `crux_core::operation::Stream`.
///
/// Sending an operation with the wrong `Command` constructor is then a compile
/// error.
///
/// ```rust
/// use crux_core::macros::Operation;
/// use facet::Facet;
/// use serde::{Deserialize, Serialize};
///
/// /// Told to the shell, never answered.
/// #[derive(Operation, Facet, Debug, Clone, Serialize, Deserialize)]
/// #[operation(notify)]
/// pub struct Publish(pub Vec<u8>);
///
/// /// Answered exactly once.
/// #[derive(Operation, Facet, Debug, Clone, Serialize, Deserialize)]
/// #[operation(request, output = GetResult, register(StoreError))]
/// pub struct Get {
///     pub key: String,
/// }
///
/// /// Answered a sequence of times.
/// #[derive(Operation, Facet, Debug, Clone, Serialize, Deserialize)]
/// #[operation(stream, output = Vec<u8>)]
/// pub struct Subscribe;
///
/// #[derive(Facet, Debug, Clone, Serialize, Deserialize)]
/// #[repr(C)]
/// pub enum GetResult {
///     Ok(Vec<u8>),
///     Err(StoreError),
/// }
///
/// #[derive(Facet, Debug, Clone, Serialize, Deserialize)]
/// #[repr(C)]
/// pub enum StoreError {
///     NotFound,
/// }
/// ```
///
/// # Registering extra types
///
/// `register(A, B, ..)` names further types that type generation should emit
/// alongside the operation and its output — types the output only mentions
/// behind a `Vec`, an `Option` or another indirection the tracer cannot see
/// through. It generates overrides of `Operation::register_types` and
/// `Operation::register_types_facet`,
/// each behind the `typegen` and `facet_typegen` **features of the crate the
/// derive is used in**, mirroring the gates on the trait's own methods.
///
/// A crate that declares neither feature still compiles: the generated `impl`
/// carries `#[allow(unexpected_cfgs)]`, so an undeclared feature name is not a
/// warning. The overrides are simply never emitted there — which is what you
/// want, since a crate with no type generation feature does no type
/// generation. To have `register(..)` take effect, declare a feature of the
/// matching name that forwards to `crux_core`, as `crux_kv` and `crux_time`
/// do:
///
/// ```toml
/// [features]
/// typegen = ["crux_core/typegen"]
/// facet_typegen = ["crux_core/facet_typegen"]
/// ```
#[proc_macro_derive(Operation, attributes(operation))]
pub fn operation(input: TokenStream) -> TokenStream {
    operation::macro_impl::operation_impl(&parse_macro_input!(input)).into()
}

#[proc_macro_derive(Export)]
#[proc_macro_error]
pub fn export(input: TokenStream) -> TokenStream {
    export_impl(&parse_macro_input!(input)).into()
}

/// Deprecated: use the `effect` attribute macro instead.
#[proc_macro_derive(Capability)]
#[proc_macro_error]
pub fn capability(input: TokenStream) -> TokenStream {
    capability_impl(&parse_macro_input!(input)).into()
}

#[cfg(test)]
fn pretty_print(ts: &proc_macro2::TokenStream) -> String {
    let file = syn::parse_file(&ts.to_string()).unwrap();
    prettyplease::unparse(&file)
}
