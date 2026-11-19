use flutter_rust_bridge::frb;
use shared::CoreFFI;
use std::sync::Mutex;

/// Opaque handle to the Crux core (same pattern as the Weather Flutter shell).
#[frb(opaque)]
pub struct CruxCore {
    inner: Mutex<CoreFFI>,
}

impl CruxCore {
    #[frb(sync)]
    pub fn new() -> CruxCore {
        CruxCore {
            inner: Mutex::new(CoreFFI::new()),
        }
    }

    pub fn update(&self, data: Vec<u8>) -> Vec<u8> {
        self.inner.lock().expect("core lock poisoned").update(&data)
    }

    pub fn resolve(&self, id: u32, data: Vec<u8>) -> Vec<u8> {
        self.inner
            .lock()
            .expect("core lock poisoned")
            .resolve(id, &data)
    }

    pub fn view(&self) -> Vec<u8> {
        self.inner.lock().expect("core lock poisoned").view()
    }
}
