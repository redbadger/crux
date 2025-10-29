#[cfg(not(target_family = "wasm"))]
pub mod uniffi_ffi {
    use std::sync::Arc;

    use crux_core::{
        bridge::EffectId,
        middleware::{BincodeFfiFormat, Bridge, Layer as _},
        Core,
    };

    use crate::App;

    /// For the Shell to provide
    #[uniffi::export(with_foreign)]
    pub trait CruxShell: Send + Sync {
        /// Called when any effects resulting from an asynchronous process
        /// need processing by the shell.
        ///
        /// The bytes are a serialized vector of requests
        fn process_effects(&self, bytes: Vec<u8>);
    }

    /// The main interface used by the shell
    #[derive(uniffi::Object)]
    pub struct CoreFFI {
        core: Bridge<Core<App>, BincodeFfiFormat>,
    }

    #[uniffi::export]
    #[allow(clippy::missing_panics_doc)]
    impl CoreFFI {
        #[uniffi::constructor]
        pub fn new(shell: Arc<dyn CruxShell>) -> Self {
            let core =
                Core::<App>::new().bridge::<BincodeFfiFormat>(
                    move |effect_bytes| match effect_bytes {
                        Ok(effect) => shell.process_effects(effect),
                        Err(e) => panic!("{e}"),
                    },
                );

            Self { core }
        }

        #[must_use]
        pub fn update(&self, data: &[u8]) -> Vec<u8> {
            let mut effects = Vec::new();
            match self.core.update(data, &mut effects) {
                Ok(()) => effects,
                Err(e) => panic!("{e}"),
            }
        }

        #[must_use]
        pub fn resolve(&self, effect_id: u32, data: &[u8]) -> Vec<u8> {
            let mut effects = Vec::new();
            match self.core.resolve(EffectId(effect_id), data, &mut effects) {
                Ok(()) => effects,
                Err(e) => panic!("{e}"),
            }
        }

        #[must_use]
        pub fn view(&self) -> Vec<u8> {
            let mut view_model = Vec::new();
            match self.core.view(&mut view_model) {
                Ok(()) => view_model,
                Err(e) => panic!("{e}"),
            }
        }
    }
}

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
pub mod wasm_ffi {
    use crux_core::middleware::{BincodeFfiFormat, Bridge, Layer as _};
    use crux_core::{bridge::EffectId, Core};

    use crate::App;

    /// The main interface used by the shell
    #[wasm_bindgen::prelude::wasm_bindgen]
    pub struct CoreFFI {
        core: Bridge<Core<App>, BincodeFfiFormat>,
    }

    struct JsCallback(js_sys::Function);

    unsafe impl Send for JsCallback {}
    unsafe impl Sync for JsCallback {}

    impl std::ops::Deref for JsCallback {
        type Target = js_sys::Function;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    #[wasm_bindgen::prelude::wasm_bindgen]
    impl CoreFFI {
        #[wasm_bindgen::prelude::wasm_bindgen(constructor)]
        pub fn new(callback: js_sys::Function) -> Self {
            use js_sys::wasm_bindgen::JsValue;

            let callback = JsCallback(callback);
            let core =
                Core::<App>::new().bridge::<BincodeFfiFormat>(
                    move |effect_bytes| match effect_bytes {
                        Ok(bytes) => {
                            callback
                                .call1(&JsValue::NULL, &JsValue::from(bytes))
                                .expect("Could not call JS callback");
                        }
                        Err(e) => {
                            panic!("{e}");
                        }
                    },
                );

            Self { core }
        }

        pub fn update(&self, data: &[u8]) -> Vec<u8> {
            let mut effects = Vec::new();
            match self.core.update(data, &mut effects) {
                Ok(()) => effects,
                Err(e) => panic!("{e}"),
            }
        }

        pub fn resolve(&self, effect_id: u32, data: &[u8]) -> Vec<u8> {
            let mut effects = Vec::new();
            match self.core.resolve(EffectId(effect_id), data, &mut effects) {
                Ok(()) => effects,
                Err(e) => panic!("{e}"),
            }
        }

        pub fn view(&self) -> Vec<u8> {
            let mut view_model = Vec::new();
            match self.core.view(&mut view_model) {
                Ok(()) => view_model,
                Err(e) => panic!("{e}"),
            }
        }
    }
}
