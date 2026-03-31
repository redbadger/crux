use wasm_bindgen::prelude::*;

use shared::location::{
    Location,
    capability::{LocationOperation, LocationResult},
};

#[allow(clippy::future_not_send)] // WASM is single-threaded
pub async fn handle(operation: &LocationOperation) -> LocationResult {
    match operation {
        LocationOperation::IsLocationEnabled => {
            let enabled = web_sys::window()
                .and_then(|w| w.navigator().geolocation().ok())
                .is_some();
            LocationResult::Enabled(enabled)
        }
        LocationOperation::GetLocation => match get_current_position().await {
            Ok((lat, lon)) => LocationResult::Location(Some(Location { lat, lon })),
            Err(_) => LocationResult::Location(None),
        },
    }
}

async fn get_current_position() -> Result<(f64, f64), JsValue> {
    let (sender, receiver) = oneshot();

    let success_sender = sender.clone();
    let success_callback = Closure::once(move |position: JsValue| {
        let coords = js_sys::Reflect::get(&position, &"coords".into()).unwrap();
        let lat = js_sys::Reflect::get(&coords, &"latitude".into())
            .unwrap()
            .as_f64()
            .unwrap_or(0.0);
        let lon = js_sys::Reflect::get(&coords, &"longitude".into())
            .unwrap()
            .as_f64()
            .unwrap_or(0.0);
        success_sender.send(Ok((lat, lon)));
    });

    let error_callback = Closure::once(move |_err: JsValue| {
        sender.send(Err(JsValue::from_str("geolocation error")));
    });

    let geolocation = web_sys::window()
        .ok_or_else(|| JsValue::from_str("no window"))?
        .navigator()
        .geolocation()
        .map_err(|_| JsValue::from_str("no geolocation"))?;

    geolocation.get_current_position_with_error_callback(
        success_callback.as_ref().unchecked_ref(),
        Some(error_callback.as_ref().unchecked_ref()),
    )?;

    // Keep closures alive
    success_callback.forget();
    error_callback.forget();

    receiver.await
}

// Simple oneshot channel using Rc<RefCell<>>
fn oneshot<T>() -> (Sender<T>, Receiver<T>) {
    let shared = std::rc::Rc::new(std::cell::RefCell::new(OneshotState {
        value: None,
        waker: None,
    }));
    (
        Sender {
            shared: shared.clone(),
        },
        Receiver { shared },
    )
}

struct OneshotState<T> {
    value: Option<T>,
    waker: Option<std::task::Waker>,
}

#[derive(Clone)]
struct Sender<T> {
    shared: std::rc::Rc<std::cell::RefCell<OneshotState<T>>>,
}

impl<T> Sender<T> {
    fn send(self, value: T) {
        let mut state = self.shared.borrow_mut();
        state.value = Some(value);
        if let Some(waker) = state.waker.take() {
            waker.wake();
        }
    }
}

struct Receiver<T> {
    shared: std::rc::Rc<std::cell::RefCell<OneshotState<T>>>,
}

impl<T> std::future::Future for Receiver<T> {
    type Output = T;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let mut state = self.shared.borrow_mut();
        if let Some(value) = state.value.take() {
            std::task::Poll::Ready(value)
        } else {
            state.waker = Some(cx.waker().clone());
            std::task::Poll::Pending
        }
    }
}
