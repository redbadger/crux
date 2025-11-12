#[cfg(not(target_family = "wasm"))]
pub mod uniffi_ffi {
    use std::sync::Arc;

    use crux_core::{
        Core,
        bridge::EffectId,
        macros::effect,
        middleware::{BincodeFfiFormat, Bridge, HandleEffectLayer, Layer as _, MapEffectLayer},
        render::RenderOperation,
    };
    use crux_http::protocol::HttpRequest;

    use crate::{App, middleware::RngMiddleware, sse::SseRequest};

    #[effect(facet_typegen)]
    pub enum Effect {
        Render(RenderOperation),
        Http(HttpRequest),
        ServerSentEvents(SseRequest),
    }

    impl From<crate::app::Effect> for Effect {
        fn from(effect: crate::app::Effect) -> Self {
            match effect {
                crate::Effect::Render(request) => Effect::Render(request),
                crate::Effect::Http(request) => Effect::Http(request),
                crate::Effect::ServerSentEvents(request) => Effect::ServerSentEvents(request),
                crate::Effect::Random(_) => panic!("Encountered a Random effect"),
            }
        }
    }

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
        core: Bridge<
            MapEffectLayer<HandleEffectLayer<Core<App>, RngMiddleware>, Effect>,
            BincodeFfiFormat,
        >,
    }

    #[uniffi::export]
    #[allow(clippy::missing_panics_doc)]
    impl CoreFFI {
        #[uniffi::constructor]
        pub fn new(shell: Arc<dyn CruxShell>) -> Self {
            let core = Core::<App>::new()
                .handle_effects_using(RngMiddleware::new())
                .map_effect::<Effect>()
                .bridge::<BincodeFfiFormat>(move |effect_bytes| match effect_bytes {
                    Ok(effect) => shell.process_effects(effect),
                    Err(e) => panic!("{e}"),
                });

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
    use crux_core::middleware::{BincodeFfiFormat, Layer as _};
    use crux_core::{Core, bridge::EffectId};

    use crate::App;

    /// The main interface used by the shell
    #[wasm_bindgen::prelude::wasm_bindgen]
    pub struct CoreFFI {
        core: crux_core::middleware::Bridge<Core<App>, BincodeFfiFormat>,
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
            match self.core.update(data) {
                Ok(effects) => effects,
                Err(e) => panic!("{e}"),
            }
        }

        pub fn resolve(&self, effect_id: u32, data: &[u8]) -> Vec<u8> {
            match self.core.resolve(EffectId(effect_id), data) {
                Ok(effects) => effects,
                Err(e) => panic!("{e}"),
            }
        }

        pub fn view(&self) -> Vec<u8> {
            match self.core.view() {
                Ok(view) => view,
                Err(e) => panic!("{e}"),
            }
        }
    }
}

#[cfg(all(target_os = "wasi", target_env = "p2"))]
pub mod wasip2 {
    use crux_core::{
        Core,
        bridge::EffectId,
        middleware::{Bridge, FfiFormat, Layer as _},
        type_generation::facet::TypeRegistry,
    };
    use serde_json::{Deserializer, Serializer, de::SliceRead};

    use crate::App;
    use exports::crux::shared_lib::core::{Guest, GuestInstance};

    wit_bindgen::generate!();
    export!(Component);

    pub struct Component;

    impl Guest for Component {
        type Instance = CoreFFI;
    }

    /// The main interface used by the shell
    pub struct CoreFFI {
        core: Bridge<Core<App>, JsonFfiFormat>,
    }

    impl GuestInstance for CoreFFI {
        fn new() -> Self {
            let core = Core::<App>::new().bridge::<JsonFfiFormat>(|_| {});

            Self { core }
        }

        fn update(&self, data: Vec<u8>) -> Result<Vec<u8>, String> {
            self.core.update(&data).map_err(|e| e.to_string())
        }

        fn resolve(&self, effect_id: u32, data: Vec<u8>) -> Result<Vec<u8>, String> {
            self.core
                .resolve(EffectId(effect_id), &data)
                .map_err(|e| e.to_string())
        }

        fn view(&self) -> Result<Vec<u8>, String> {
            self.core.view().map_err(|e| e.to_string())
        }

        fn schema(&self) -> String {
            let registry = TypeRegistry::new().register_app::<App>().build().registry();

            format!("{registry:#?}")
        }
    }

    pub struct JsonFfiFormat;

    impl FfiFormat for JsonFfiFormat {
        type Serializer<'b> = Serializer<&'b mut Vec<u8>>;
        type Deserializer<'b> = Deserializer<SliceRead<'b>>;

        fn serializer(buffer: &mut Vec<u8>) -> Serializer<&mut Vec<u8>> {
            Serializer::new(buffer)
        }

        fn deserializer(bytes: &[u8]) -> Deserializer<SliceRead<'_>> {
            Deserializer::from_slice(bytes)
        }
    }
}
