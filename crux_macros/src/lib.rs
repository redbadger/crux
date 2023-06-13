mod capability;
mod effect;
mod export;

use capability::capability_impl;
use effect::effect_impl;
use export::export_impl;
use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;
use syn::parse_macro_input;

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
