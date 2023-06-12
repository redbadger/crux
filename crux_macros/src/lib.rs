mod capability;
mod effect;
mod export;

use capability::capability_impl;
use effect::effect_impl;
use export::export_impl;
use proc_macro::TokenStream;
use syn::parse_macro_input;

#[proc_macro_derive(Effect, attributes(effect))]
pub fn effect(input: TokenStream) -> TokenStream {
    effect_impl(&parse_macro_input!(input)).into()
}

#[proc_macro_derive(Export)]
pub fn export(input: TokenStream) -> TokenStream {
    export_impl(&parse_macro_input!(input)).into()
}

#[proc_macro_derive(Capability)]
pub fn capability(input: TokenStream) -> TokenStream {
    capability_impl(&parse_macro_input!(input)).into()
}
