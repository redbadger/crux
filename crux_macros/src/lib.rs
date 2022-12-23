mod generate_effect;

use generate_effect::{impl_generate_effect, GenerateEffectAttr};
use syn::{parse_macro_input, ItemStruct};

#[proc_macro_attribute]
pub fn generate_effect(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let attr = parse_macro_input!(attr as GenerateEffectAttr);
    let item = parse_macro_input!(item as ItemStruct);

    let output = impl_generate_effect(attr, item);

    proc_macro::TokenStream::from(output)
}
