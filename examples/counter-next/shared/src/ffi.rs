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
            match self.core.update(data) {
                Ok(effects) => effects,
                Err(e) => panic!("{e}"),
            }
        }

        #[must_use]
        pub fn resolve(&self, effect_id: u32, data: &[u8]) -> Vec<u8> {
            match self.core.resolve(EffectId(effect_id), data) {
                Ok(effects) => effects,
                Err(e) => panic!("{e}"),
            }
        }

        #[must_use]
        pub fn view(&self) -> Vec<u8> {
            match self.core.view() {
                Ok(view) => view,
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
    use crux_core::{Core, bridge::Bridge};
    use std::sync::OnceLock;

    use crate::{App, bindings};

    /// The main interface used by the shell
    pub struct CoreFFI {
        core: Bridge<App>,
    }

    impl CoreFFI {
        pub fn new() -> Self {
            let core = Bridge::new(Core::new());

            Self { core }
        }

        #[must_use]
        pub fn update(&self, data: &[u8]) -> Vec<u8> {
            match self.core.process_event(data) {
                Ok(effects) => effects,
                Err(e) => panic!("{e}"),
            }
        }

        #[must_use]
        pub fn resolve(&self, effect_id: u32, data: &[u8]) -> Vec<u8> {
            match self.core.handle_response(effect_id, data) {
                Ok(effects) => effects,
                Err(e) => panic!("{e}"),
            }
        }

        #[must_use]
        pub fn view(&self) -> Vec<u8> {
            match self.core.view() {
                Ok(view) => view,
                Err(e) => panic!("{e}"),
            }
        }
    }

    static CORE: OnceLock<CoreFFI> = OnceLock::new();

    fn get_core() -> &'static CoreFFI {
        CORE.get_or_init(|| CoreFFI::new())
    }

    pub struct Component;

    impl bindings::Guest for Component {
        fn update(data: Vec<u8>) -> Vec<u8> {
            get_core().update(&data)
        }

        fn resolve(effect_id: u32, data: Vec<u8>) -> Vec<u8> {
            get_core().resolve(effect_id, &data)
        }

        fn view() -> Vec<u8> {
            get_core().view()
        }
    }

    bindings::export!(Component with_types_in bindings);
}
