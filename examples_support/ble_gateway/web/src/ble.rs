//! The gateway's `BleConnect` and `BleWrite` effects, with Web Bluetooth.
//!
//! Web Bluetooth is a JS-object API with event handlers, so the connection lives in a
//! thread local (wasm is single-threaded) and notifications are pushed into a channel
//! that the effect handler drains into `core.resolve`.

use std::cell::RefCell;

use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};
use gateway_core::{BleConnect, BleEvent, BleWrite, ble::WriteResult};
use js_sys::{JsString, Uint8Array};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use web_sys::{
    BluetoothDevice, BluetoothLeScanFilterInit, BluetoothRemoteGattCharacteristic,
    RequestDeviceOptions,
};

use crate::describe;

thread_local! {
    /// The device last connected to, so a reconnect needs no chooser (and no click).
    static DEVICE: RefCell<Option<BluetoothDevice>> = const { RefCell::new(None) };
    /// The characteristic `BleWrite` writes to, while connected.
    static RX: RefCell<Option<BluetoothRemoteGattCharacteristic>> = const { RefCell::new(None) };
}

/// Start connecting; the returned channel carries the `BleConnect` stream's events and
/// closes after `Disconnected` or `Failed`.
pub fn connect(operation: &BleConnect) -> UnboundedReceiver<BleEvent> {
    let (events, receiver) = unbounded();
    let operation = operation.clone();
    wasm_bindgen_futures::spawn_local(async move {
        if let Err(error) = open(&operation, events.clone()).await {
            // A remembered device that can't be reached any more should not block the
            // chooser next time.
            DEVICE.with_borrow_mut(Option::take);
            let _ = events.unbounded_send(BleEvent::Failed(describe(&error)));
            events.close_channel();
        }
    });
    receiver
}

#[allow(clippy::future_not_send)] // WASM is single-threaded
async fn open(operation: &BleConnect, events: UnboundedSender<BleEvent>) -> Result<(), JsValue> {
    let remembered = DEVICE.with_borrow(Clone::clone);
    let device = if let Some(device) = remembered {
        device
    } else {
        let bluetooth = web_sys::window()
            .and_then(|window| window.navigator().bluetooth())
            .ok_or_else(|| JsValue::from_str("Web Bluetooth is not available (use Chrome)"))?;
        let filter = BluetoothLeScanFilterInit::new();
        filter.set_services(&[JsString::from(operation.service.as_str())]);
        let options = RequestDeviceOptions::new();
        options.set_filters(&[filter]);
        // Needs the user's click (transient activation): the Connect button.
        bluetooth.request_device(&options).await?
    };

    // `set_on…` replaces any handler from an earlier connection to the same device.
    let disconnected = {
        let events = events.clone();
        Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
            RX.with_borrow_mut(Option::take);
            let _ = events.unbounded_send(BleEvent::Disconnected);
            events.close_channel();
        })
    };
    device.set_ongattserverdisconnected(Some(disconnected.as_ref().unchecked_ref()));
    // One small closure per connection, kept for the page's lifetime (dropping it from
    // inside its own call would be an error).
    disconnected.forget();

    let gatt = device
        .gatt()
        .ok_or_else(|| JsValue::from_str("device has no GATT server"))?;
    let server = gatt.connect().await?;
    let service = server
        .get_primary_service_with_str(&operation.service)
        .await?;
    let tx = service.get_characteristic_with_str(&operation.tx).await?;
    let rx = service.get_characteristic_with_str(&operation.rx).await?;

    let notified = {
        let events = events.clone();
        Closure::<dyn FnMut(web_sys::Event)>::new(move |event: web_sys::Event| {
            let Some(value) = event
                .target()
                .and_then(|target| target.dyn_into::<BluetoothRemoteGattCharacteristic>().ok())
                .and_then(|characteristic| characteristic.value())
            else {
                return;
            };
            // wasm32: usize is 32 bits, so these conversions cannot fail.
            let bytes = Uint8Array::new_with_byte_offset_and_length(
                &value.buffer(),
                u32::try_from(value.byte_offset()).unwrap_or_default(),
                u32::try_from(value.byte_length()).unwrap_or_default(),
            )
            .to_vec();
            let _ = events.unbounded_send(BleEvent::Data(bytes));
        })
    };
    tx.set_oncharacteristicvaluechanged(Some(notified.as_ref().unchecked_ref()));
    notified.forget();

    RX.with_borrow_mut(|slot| *slot = Some(rx));
    let name = device.name().unwrap_or_else(|| "(unnamed)".to_string());
    DEVICE.with_borrow_mut(|slot| *slot = Some(device));

    // `Connected` goes first, so the core has reset its state before any data arrives;
    // the device only notifies once we have subscribed.
    let _ = events.unbounded_send(BleEvent::Connected { name });
    tx.start_notifications().await?;
    Ok(())
}

/// Write the frame chunk by chunk, each write finishing before the next starts.
#[allow(clippy::future_not_send)] // WASM is single-threaded
pub async fn write(operation: &BleWrite) -> WriteResult {
    let rx = RX
        .with_borrow(Clone::clone)
        .ok_or_else(|| "not connected".to_string())?;
    for chunk in operation.data.chunks(operation.chunk.max(1)) {
        rx.write_value_without_response_with_u8_slice(chunk)
            .map_err(|error| describe(&error))?
            .await
            .map_err(|error| describe(&error))?;
    }
    Ok(())
}
