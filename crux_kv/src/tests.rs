use anyhow::Result;
use crux_core::{
    Command,
    macros::effect,
    render::{RenderOperation, render},
    testing::AppTester,
};
use serde::{Deserialize, Serialize};

use crate::{KeyValueOperation, KeyValueResponse, KeyValueResult, command::KeyValue};

#[derive(Default)]
pub struct App;

#[derive(Debug, Serialize, Deserialize)]
pub enum Event {
    Get,
    Set,
    Delete,
    Exists,
    ListKeys,
    GetThenSet,

    GetResponse(KeyValueResult),
    SetResponse(KeyValueResult),
    ExistsResponse(KeyValueResult),
    ListKeysResponse(KeyValueResult),
}

#[derive(Debug, Default)]
pub struct Model {
    pub value: i32,
    pub keys: Vec<String>,
    pub cursor: u64,
    pub successful: bool,
}

#[derive(Serialize, Deserialize, Default)]
pub struct ViewModel {
    pub result: String,
}

impl crux_core::App for App {
    type Event = Event;
    type Model = Model;
    type ViewModel = ViewModel;

    type Capabilities = ();
    type Effect = Effect;

    fn update(&self, event: Event, model: &mut Model, _caps: &()) -> Command<Effect, Event> {
        let key = "test".to_string();
        match event {
            Event::Get => KeyValue::get(key).then_send(Event::GetResponse),
            Event::Set => {
                KeyValue::set(key, 42i32.to_ne_bytes().to_vec()).then_send(Event::SetResponse)
            }
            Event::Delete => KeyValue::delete(key).then_send(Event::SetResponse),
            Event::Exists => KeyValue::exists(key).then_send(Event::ExistsResponse),
            Event::ListKeys => {
                KeyValue::list_keys("test:".to_string(), 0).then_send(Event::ListKeysResponse)
            }

            Event::GetThenSet => Command::new(|ctx| async move {
                let result = KeyValue::get("test_num".to_string())
                    .into_future(ctx.clone())
                    .await;
                let Ok(Some(value)) = result.unwrap_get() else {
                    panic!("expected get response with a value");
                };

                let num = i32::from_ne_bytes(value.try_into().unwrap());
                let result =
                    KeyValue::set("test_num".to_string(), (num + 1).to_ne_bytes().to_vec())
                        .into_future(ctx.clone())
                        .await;

                ctx.send_event(Event::SetResponse(result));
            }),

            Event::GetResponse(KeyValueResult::Ok(KeyValueResponse::Get(Some(value)))) => {
                let (int_bytes, _rest) = value.split_at(std::mem::size_of::<i32>());
                model.value = i32::from_ne_bytes(int_bytes.try_into().unwrap());

                Command::done()
            }

            Event::SetResponse(KeyValueResult::Ok(_response)) => {
                model.successful = true;

                render()
            }

            Event::ExistsResponse(KeyValueResult::Ok(_response)) => {
                model.successful = true;
                render()
            }

            Event::ListKeysResponse(KeyValueResult::Ok(KeyValueResponse::ListKeys {
                keys,
                next_cursor,
            })) => {
                model.keys = keys;
                model.cursor = next_cursor;

                render()
            }
            Event::GetResponse(KeyValueResult::Ok(_))
            | Event::ListKeysResponse(KeyValueResult::Ok(_)) => panic!("expected value"),

            Event::GetResponse(KeyValueResult::Err(error))
            | Event::SetResponse(KeyValueResult::Err(error))
            | Event::ExistsResponse(KeyValueResult::Err(error))
            | Event::ListKeysResponse(KeyValueResult::Err(error)) => {
                panic!("Error: {error:?}");
            }
        }
    }

    fn view(&self, model: &Self::Model) -> Self::ViewModel {
        ViewModel {
            result: format!("Success: {}, Value: {}", model.successful, model.value),
        }
    }
}

#[effect]
pub enum Effect {
    KeyValue(KeyValueOperation),
    Render(RenderOperation),
}

#[test]
fn test_get() {
    let app = AppTester::<App>::default();
    let mut model = Model::default();

    let request = &mut app
        .update(Event::Get, &mut model)
        .expect_one_effect()
        .expect_key_value();

    assert_eq!(
        request.operation,
        KeyValueOperation::Get {
            key: "test".to_string()
        }
    );

    let _updated = app.resolve_to_event_then_update(
        request,
        KeyValueResult::Ok(KeyValueResponse::Get(42i32.to_ne_bytes().to_vec().into())),
        &mut model,
    );

    assert_eq!(model.value, 42);
}

#[test]
fn test_set() {
    let app = AppTester::<App>::default();
    let mut model = Model::default();

    let request = &mut app
        .update(Event::Set, &mut model)
        .expect_one_effect()
        .expect_key_value();

    assert_eq!(
        request.operation,
        KeyValueOperation::Set {
            key: "test".to_string(),
            value: 42i32.to_ne_bytes().to_vec(),
        }
    );

    let _updated = app.resolve_to_event_then_update(
        request,
        KeyValueResult::Ok(KeyValueResponse::Set(None)),
        &mut model,
    );

    assert!(model.successful);
}

#[test]
fn test_delete() {
    let app = AppTester::<App>::default();
    let mut model = Model::default();

    let request = &mut app
        .update(Event::Delete, &mut model)
        .expect_one_effect()
        .expect_key_value();

    assert_eq!(
        request.operation,
        KeyValueOperation::Delete {
            key: "test".to_string()
        }
    );

    let _updated = app.resolve_to_event_then_update(
        request,
        KeyValueResult::Ok(KeyValueResponse::Delete(None)),
        &mut model,
    );

    assert!(model.successful);
}

#[test]
fn test_exists() {
    let app = AppTester::<App>::default();
    let mut model = Model::default();

    let request = &mut app
        .update(Event::Exists, &mut model)
        .expect_one_effect()
        .expect_key_value();

    assert_eq!(
        request.operation,
        KeyValueOperation::Exists {
            key: "test".to_string()
        }
    );

    let _updated = app.resolve_to_event_then_update(
        request,
        KeyValueResult::Ok(KeyValueResponse::Exists(true)),
        &mut model,
    );

    assert!(model.successful);
}

#[test]
fn test_list_keys() {
    let app = AppTester::<App>::default();
    let mut model = Model::default();

    let request = &mut app
        .update(Event::ListKeys, &mut model)
        .expect_one_effect()
        .expect_key_value();

    assert_eq!(
        request.operation,
        KeyValueOperation::ListKeys {
            prefix: "test:".to_string(),
            cursor: 0,
        }
    );

    let _updated = app.resolve_to_event_then_update(
        request,
        KeyValueResult::Ok(KeyValueResponse::ListKeys {
            keys: vec!["test:1".to_string(), "test:2".to_string()],
            next_cursor: 2,
        }),
        &mut model,
    );

    assert_eq!(model.keys, vec!["test:1".to_string(), "test:2".to_string()]);
    assert_eq!(model.cursor, 2);
}

#[test]
pub fn test_kv_async() {
    let app = AppTester::<App>::default();
    let mut model = Model::default();

    let request = &mut app
        .update(Event::GetThenSet, &mut model)
        .expect_one_effect()
        .expect_key_value();

    assert_eq!(
        request.operation,
        KeyValueOperation::Get {
            key: "test_num".to_string()
        }
    );

    let request = &mut app
        .resolve(
            request,
            KeyValueResult::Ok(KeyValueResponse::Get(17u32.to_ne_bytes().to_vec().into())),
        )
        .unwrap()
        .expect_one_effect()
        .expect_key_value();

    assert_eq!(
        request.operation,
        KeyValueOperation::Set {
            key: "test_num".to_string(),
            value: 18u32.to_ne_bytes().to_vec(),
        }
    );

    let _updated = app.resolve_to_event_then_update(
        request,
        KeyValueResult::Ok(KeyValueResponse::Set(None)),
        &mut model,
    );

    assert!(model.successful);
}

#[test]
fn test_kv_operation_debug_repr() {
    {
        // get
        let op = KeyValueOperation::Get {
            key: "my key".into(),
        };
        let repr = format!("{op:?}");
        assert_eq!(repr, r#"Get { key: "my key" }"#);
    }

    {
        // set small
        let op = KeyValueOperation::Set {
            key: "my key".into(),
            value: b"my value".to_vec(),
        };
        let repr = format!("{op:?}");
        assert_eq!(repr, r#"Set { key: "my key", value: "my value" }"#);
    }

    {
        // set big
        let op = KeyValueOperation::Set {
            key: "my key".into(),
            value:
                // we check that we handle unicode boundaries correctly
                "abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstu😀😀😀😀😀😀".as_bytes().to_vec(),
        };
        let repr = format!("{op:?}");
        assert_eq!(
            repr,
            r#"Set { key: "my key", value: "abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstu😀😀"... }"#
        );
    }

    {
        // set binary
        let op = KeyValueOperation::Set {
            key: "my key".into(),
            value: vec![255, 255],
        };
        let repr = format!("{op:?}");
        assert_eq!(
            repr,
            r#"Set { key: "my key", value: <binary data - 2 bytes> }"#
        );
    }
}
