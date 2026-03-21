use std::{cell::RefCell, rc::Rc};

use base64::{Engine, engine::general_purpose::STANDARD};
use wasm_bindgen::{JsCast, closure::Closure};

use shared::{
    ViewModel,
    capabilities::pub_sub::{Message, PubSubOperation},
};

use crate::core::{self, Core};

use leptos::prelude::*;

pub fn handle(
    core: &Core,
    request: crux_core::Request<PubSubOperation>,
    render: WriteSignal<ViewModel>,
) {
    match &request.operation {
        PubSubOperation::Publish(data) => {
            if let Ok(channel) = web_sys::BroadcastChannel::new("crux-notes") {
                let encoded = STANDARD.encode(data);
                let _ = channel.post_message(&wasm_bindgen::JsValue::from_str(&encoded));
                channel.close();
            }
        }
        PubSubOperation::Subscribe => {
            let core = core.clone();
            let request = Rc::new(RefCell::new(request));

            if let Ok(channel) = web_sys::BroadcastChannel::new("crux-notes") {
                let closure = Closure::<dyn Fn(web_sys::MessageEvent)>::new(
                    move |event: web_sys::MessageEvent| {
                        if let Some(data_str) = event.data().as_string()
                            && let Ok(bytes) = STANDARD.decode(&data_str)
                        {
                            let mut req = request.borrow_mut();
                            let response = Message(bytes);

                            for effect in core.resolve(&mut *req, response).expect("should resolve")
                            {
                                core::process_effect(&core, effect, render);
                            }
                        }
                    },
                );
                channel.set_onmessage(Some(closure.as_ref().unchecked_ref()));
                closure.forget();
            }
        }
    }
}
