mod capability;
mod effect;
mod export;

use capability::capability_impl;
use effect::effect_impl;
use export::export_impl;
use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;
use syn::parse_macro_input;

/// Procedural macro to derive an Effect enum, with a variant for
/// each non-skipped capability.
///
/// The default name of the Effect enum is "Effect", but this can be
/// overridden with the `name` attribute.
///
/// The default name of the app struct is "App", but this can be
/// overridden with the `app` attribute.
///
/// No Effect variant will be generated for fields annotated with
/// `#[effect(skip)]`.
///
/// e.g.
/// ```rust
/// # use crux_core::{Capability, render::Render, compose::Compose};
/// # use crux_core::macros::Effect;
/// # #[derive(Default)]
/// # struct MyApp;
/// # pub enum MyEvent {None}
/// # impl crux_core::App for MyApp {
/// #     type Event = MyEvent;
/// #     type Model = ();
/// #     type ViewModel = ();
/// #     type Capabilities = MyCapabilities;
/// #     fn update(
/// #         &self,
/// #         _event: Self::Event,
/// #         _model: &mut Self::Model,
/// #         _caps: &Self::Capabilities,
/// #     ) {
/// #         unimplemented!()
/// #     }
/// #     fn view(&self, _model: &Self::Model) -> Self::ViewModel {
/// #         unimplemented!()
/// #     }
/// # }
/// #[derive(Effect)]
/// #[effect(name = "MyEffect", app = "MyApp")]
/// pub struct MyCapabilities {
///     pub http: crux_http::Http<MyEvent>,
///     pub render: Render<MyEvent>,
///     #[effect(skip)]
///     pub compose: Compose<MyEvent>,
/// }

#[proc_macro_derive(Effect, attributes(effect))]
#[proc_macro_error]
pub fn effect(input: TokenStream) -> TokenStream {
    effect_impl(&parse_macro_input!(input)).into()
}

#[proc_macro_derive(Export)]
#[proc_macro_error]
pub fn export(input: TokenStream) -> TokenStream {
    export_impl(&parse_macro_input!(input)).into()
}

#[proc_macro_derive(Capability)]
#[proc_macro_error]
pub fn capability(input: TokenStream) -> TokenStream {
    capability_impl(&parse_macro_input!(input)).into()
}
