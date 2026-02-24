#![deny(clippy::pedantic)]

mod capability;
mod effect;
mod export;

use capability::capability_impl;
use effect::macro_impl::EffectArgs;
use export::export_impl;
use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;
use syn::{ItemEnum, parse_macro_input};

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
    let args = parse_macro_input!(args as EffectArgs);
    let input = parse_macro_input!(input as ItemEnum);
    effect::macro_impl::effect_impl(args, input).into()
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
