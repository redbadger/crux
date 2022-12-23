mod generate_effect;

use generate_effect::{impl_generate_effect, GenerateEffectAttr};
use proc_macro::TokenStream;
use syn::{parse_macro_input, ItemStruct};

#[proc_macro_attribute]
pub fn generate_effect(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr = parse_macro_input!(attr as GenerateEffectAttr);
    let item = parse_macro_input!(item as ItemStruct);

    let output = impl_generate_effect(attr, item);

    TokenStream::from(output)
}
