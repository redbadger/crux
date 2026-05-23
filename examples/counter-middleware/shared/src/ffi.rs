use std::sync::Arc;

use crux_core::{
    Core,
    bridge::EffectId,
    macros::effect,
    middleware::{BincodeFfiFormat, Bridge, Layer as _},
    render::RenderOperation,
};
use crux_http::protocol::HttpRequest;

#[cfg(not(target_family = "wasm"))]
use crux_core::middleware::{HandleEffectLayer, MapEffectLayer};

#[cfg(not(target_family = "wasm"))]
use crate::middleware::RngMiddleware;
use crate::{Counter, RandomNumberRequest, sse::SseRequest};

#[effect(facet_typegen)]
pub enum Effect {
    Render(RenderOperation),
    Http(HttpRequest),
    ServerSentEvents(SseRequest),
    Random(RandomNumberRequest),
}

impl From<crate::app::Effect> for Effect {
    fn from(effect: crate::app::Effect) -> Self {
        match effect {
            crate::Effect::Render(request) => Effect::Render(request),
            crate::Effect::Http(request) => Effect::Http(request),
            crate::Effect::ServerSentEvents(request) => Effect::ServerSentEvents(request),
            crate::Effect::Random(request) => Effect::Random(request),
        }
    }
}

#[cfg(not(target_family = "wasm"))]
type CoreBridge = Bridge<
    MapEffectLayer<HandleEffectLayer<Core<Counter>, RngMiddleware>, Effect>,
    BincodeFfiFormat,
>;

#[cfg(target_family = "wasm")]
type CoreBridge = Bridge<Core<Counter>, BincodeFfiFormat>;

/// For the Shell to provide.
#[boltffi::export]
#[cfg_attr(feature = "uniffi", uniffi::export(with_foreign))]
pub trait CruxShell: Send + Sync {
    /// Called when any effects resulting from an asynchronous process
    /// need processing by the shell.
    ///
    /// The bytes are a serialized vector of requests.
    fn process_effects(&self, bytes: Vec<u8>);
}

/// The main interface used by the shell.
#[cfg_attr(feature = "uniffi", derive(uniffi::Object))]
pub struct CoreFFI {
    core: CoreBridge,
}

#[boltffi::export]
#[cfg_attr(feature = "uniffi", uniffi::export)]
#[allow(clippy::missing_panics_doc)]
impl CoreFFI {
    #[cfg_attr(feature = "uniffi", uniffi::constructor)]
    pub fn new(shell: Arc<dyn CruxShell>) -> Self {
        #[cfg(not(target_family = "wasm"))]
        let core = Core::<Counter>::new()
            .handle_effects_using(RngMiddleware::new())
            .map_effect::<Effect>()
            .bridge::<BincodeFfiFormat>(move |effect_bytes| match effect_bytes {
                Ok(effect) => shell.process_effects(effect),
                Err(e) => panic!("{e}"),
            });

        #[cfg(target_family = "wasm")]
        let core =
            Core::<Counter>::new().bridge::<BincodeFfiFormat>(
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

#[cfg(feature = "wasm_bindgen")]
pub mod wasm_bindgen {
    use crux_core::middleware::{BincodeFfiFormat, Layer as _};
    use crux_core::{Core, bridge::EffectId};

    use crate::Counter;

    /// Deprecated wasm-bindgen compatibility surface used during the BoltFFI migration.
    #[wasm_bindgen::prelude::wasm_bindgen]
    pub struct CoreFFI {
        core: crux_core::middleware::Bridge<Core<Counter>, BincodeFfiFormat>,
    }

    struct JsCallback(js_sys::Function);

    #[allow(unsafe_code)]
    unsafe impl Send for JsCallback {}
    #[allow(unsafe_code)]
    unsafe impl Sync for JsCallback {}

    impl std::ops::Deref for JsCallback {
        type Target = js_sys::Function;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    #[wasm_bindgen::prelude::wasm_bindgen]
    impl CoreFFI {
        /// # Panics
        ///
        /// This function panics if the provided JavaScript callback function is not callable.
        #[wasm_bindgen::prelude::wasm_bindgen(constructor)]
        #[must_use]
        pub fn new(callback: js_sys::Function) -> Self {
            use js_sys::wasm_bindgen::JsValue;

            let callback = JsCallback(callback);
            let core = Core::<Counter>::new().bridge::<BincodeFfiFormat>(move |effect_bytes| {
                match effect_bytes {
                    Ok(bytes) => {
                        callback
                            .call1(&JsValue::NULL, &JsValue::from(bytes))
                            .expect("Could not call JS callback");
                    }
                    Err(e) => {
                        panic!("{e}");
                    }
                }
            });

            Self { core }
        }

        /// # Panics
        ///
        /// This function panics if the event is not processed successfully.
        pub fn update(&self, data: &[u8]) -> Vec<u8> {
            let mut effects = Vec::new();
            match self.core.update(data, &mut effects) {
                Ok(()) => effects,
                Err(e) => panic!("{e}"),
            }
        }

        /// # Panics
        ///
        /// This function panics if the id is not valid,
        /// or the effect response is not processed successfully.
        pub fn resolve(&self, id: u32, data: &[u8]) -> Vec<u8> {
            let mut effects = Vec::new();
            match self.core.resolve(EffectId(id), data, &mut effects) {
                Ok(()) => effects,
                Err(e) => panic!("{e}"),
            }
        }

        /// # Panics
        ///
        /// This function panics if the view model cannot be generated.
        pub fn view(&self) -> Vec<u8> {
            let mut view_model = Vec::new();
            match self.core.view(&mut view_model) {
                Ok(()) => view_model,
                Err(e) => panic!("{e}"),
            }
        }
    }
}

#[cfg(all(target_os = "wasi", target_env = "p2", feature = "wit_bindgen"))]
pub mod wit_bindgen {
    use crux_core::{
        Core,
        bridge::{EffectId, JsonFfiFormat},
        middleware::{Bridge, Layer as _},
        type_generation::facet::TypeRegistry,
    };

    use crate::Counter;
    use exports::crux::shared_lib::core::{Guest, GuestInstance};

    wit_bindgen::generate!();
    export!(Component);

    pub struct Component;

    impl Guest for Component {
        type Instance = CoreFFI;
    }

    /// The main interface used by the shell.
    pub struct CoreFFI {
        core: Bridge<Core<Counter>, JsonFfiFormat>,
    }

    impl GuestInstance for CoreFFI {
        fn new() -> Self {
            let core = Core::<Counter>::new().bridge::<JsonFfiFormat>(|_| {});

            Self { core }
        }

        fn update(&self, data: Vec<u8>) -> Result<Vec<u8>, String> {
            let mut effects = Vec::new();
            match self.core.update(&data, &mut effects) {
                Ok(()) => Ok(effects),
                Err(e) => Err(e.to_string()),
            }
        }

        fn resolve(&self, id: u32, data: Vec<u8>) -> Result<Vec<u8>, String> {
            let mut effects = Vec::new();
            match self.core.resolve(EffectId(id), &data, &mut effects) {
                Ok(()) => Ok(effects),
                Err(e) => Err(e.to_string()),
            }
        }

        fn view(&self) -> Result<Vec<u8>, String> {
            let mut view_model = Vec::new();
            match self.core.view(&mut view_model) {
                Ok(()) => Ok(view_model),
                Err(e) => Err(e.to_string()),
            }
        }

        fn schema(&self) -> String {
            let registry = TypeRegistry::new()
                .register_app::<Counter>()
                .expect("to be able to register app")
                .build()
                .expect("to be able to build registry")
                .registry();

            format!("{registry:#?}")
        }
    }
}
