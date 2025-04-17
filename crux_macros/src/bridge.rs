use proc_macro2::TokenStream;
use quote::quote;
use syn::ItemStruct;

pub fn bridge_impl(input: &ItemStruct) -> TokenStream {
    let ident = &input.ident;
    quote! {
        #[derive(Default)]
        #input

        static CORE: ::std::sync::LazyLock<::crux_core::bridge::Bridge<#ident>> =
            ::std::sync::LazyLock::new(|| {
                ::crux_core::bridge::Bridge::new(::crux_core::Core::new())
            });

        #[cfg(not(target_family = "wasm"))]
        ::uniffi::setup_scaffolding!();

        #[cfg_attr(not(target_family = "wasm"), ::uniffi::export)]
        #[cfg_attr(target_family = "wasm", ::wasm_bindgen::prelude::wasm_bindgen)]
        pub fn process_event(data: &[u8]) -> Vec<u8> {
            match CORE.process_event(data) {
                Ok(effects) => effects,
                Err(e) => panic!("{e}"),
            }
        }

        #[cfg_attr(not(target_family = "wasm"), ::uniffi::export)]
        #[cfg_attr(target_family = "wasm", ::wasm_bindgen::prelude::wasm_bindgen)]
        pub fn handle_response(id: u32, data: &[u8]) -> Vec<u8> {
            match CORE.handle_response(id, data) {
                Ok(effects) => effects,
                Err(e) => panic!("{e}"),
            }
        }

        #[cfg_attr(not(target_family = "wasm"), ::uniffi::export)]
        #[cfg_attr(target_family = "wasm", ::wasm_bindgen::prelude::wasm_bindgen)]
        pub fn view() -> Vec<u8> {
            match CORE.view() {
                Ok(view) => view,
                Err(e) => panic!("{e}"),
            }
        }
    }
}
