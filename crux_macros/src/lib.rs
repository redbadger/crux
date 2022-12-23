mod effect;

use effect::effect_impl;
use proc_macro::TokenStream;
use syn::{parse_macro_input, AttributeArgs, FieldsNamed, ItemStruct};

#[proc_macro_attribute]
pub fn effect(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr = parse_macro_input!(attr as AttributeArgs);
    let item = parse_macro_input!(item as ItemStruct);

    let output = effect_impl(attr, item);

    TokenStream::from(output)
}
