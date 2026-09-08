//! The same scenarios as `legacy`, driven through [`KeyValueStore`] and the
//! per-operation types.

use crux_core::{
    App as _, Command,
    macros::effect,
    render::{RenderOperation, render},
};
use serde::{Deserialize, Serialize};

use crate::{
    DataResult, KeyValueStore, ListResult, StatusResult,
    error::KeyValueError,
    operation::{self, BoolResult, Keys, KeysResult, ValueResult},
    protocol::Value,
};

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

    GetResponse(DataResult),
    SetResponse(DataResult),
    ExistsResponse(StatusResult),
    ListKeysResponse(ListResult),
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

    type Effect = Effect;

    fn update(&self, event: Event, model: &mut Model) -> Command<Effect, Event> {
        let key = "test".to_string();
        match event {
            Event::Get => KeyValueStore::get(key).then_send(Event::GetResponse),
            Event::Set => {
                KeyValueStore::set(key, 42i32.to_ne_bytes().to_vec()).then_send(Event::SetResponse)
            }
            Event::Delete => KeyValueStore::delete(key).then_send(Event::SetResponse),
            Event::Exists => KeyValueStore::exists(key).then_send(Event::ExistsResponse),
            Event::ListKeys => {
                KeyValueStore::list_keys("test:".to_string(), 0).then_send(Event::ListKeysResponse)
            }

            Event::GetThenSet => Command::new(|ctx| async move {
                let Result::Ok(Some(value)) = KeyValueStore::get("test_num".to_string())
                    .into_future(ctx.clone())
                    .await
                else {
                    panic!("expected get response with a value");
                };

                let num = i32::from_ne_bytes(value.try_into().unwrap());
                let result =
                    KeyValueStore::set("test_num".to_string(), (num + 1).to_ne_bytes().to_vec())
                        .into_future(ctx.clone())
                        .await;

                ctx.send_event(Event::SetResponse(result));
            }),

            Event::GetResponse(Ok(Some(value))) => {
                let (int_bytes, _rest) = value.split_at(std::mem::size_of::<i32>());
                model.value = i32::from_ne_bytes(int_bytes.try_into().unwrap());

                Command::done()
            }

            Event::GetResponse(Ok(None)) => {
                panic!("expected value");
            }

            Event::SetResponse(Ok(_response)) => {
                model.successful = true;

                render()
            }

            Event::ExistsResponse(Ok(_response)) => {
                model.successful = true;

                render()
            }

            Event::ListKeysResponse(Ok((keys, cursor))) => {
                model.keys = keys;
                model.cursor = cursor;

                render()
            }

            Event::GetResponse(Err(error))
            | Event::SetResponse(Err(error))
            | Event::ExistsResponse(Err(error))
            | Event::ListKeysResponse(Err(error)) => {
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
    Get(operation::Get),
    Set(operation::Set),
    Delete(operation::Delete),
    Exists(operation::Exists),
    ListKeys(operation::ListKeys),
    Render(RenderOperation),
}

#[test]
fn test_get() {
    let app = App;
    let mut model = Model::default();

    let mut cmd = app.update(Event::Get, &mut model);

    cmd.expect_no_events();
    let mut request = cmd.expect_one_effect().expect_get();

    assert_eq!(
        request.operation,
        operation::Get {
            key: "test".to_string()
        }
    );

    request
        .resolve(ValueResult::Ok(42i32.to_ne_bytes().to_vec().into()))
        .expect("effect should resolve");

    let event = cmd.expect_one_event();
    app.update(event, &mut model).expect_no_effect_or_events();

    assert_eq!(model.value, 42);
}

#[test]
fn test_set() {
    let app = App;
    let mut model = Model::default();

    let mut cmd = app.update(Event::Set, &mut model);

    cmd.expect_no_events();
    let mut request = cmd.expect_one_effect().expect_set();

    assert_eq!(
        request.operation,
        operation::Set {
            key: "test".to_string(),
            value: 42i32.to_ne_bytes().to_vec(),
        }
    );

    request
        .resolve(ValueResult::Ok(Value::None))
        .expect("effect should resolve");

    let event = cmd.expect_one_event();
    app.update(event, &mut model)
        .expect_one_effect()
        .expect_render();

    assert!(model.successful);
}

#[test]
fn test_delete() {
    let app = App;
    let mut model = Model::default();

    let mut cmd = app.update(Event::Delete, &mut model);

    cmd.expect_no_events();
    let mut request = cmd.expect_one_effect().expect_delete();

    assert_eq!(
        request.operation,
        operation::Delete {
            key: "test".to_string()
        }
    );

    request
        .resolve(ValueResult::Ok(Value::None))
        .expect("effect should resolve");

    let event = cmd.expect_one_event();
    app.update(event, &mut model)
        .expect_one_effect()
        .expect_render();

    assert!(model.successful);
}

#[test]
fn test_exists() {
    let app = App;
    let mut model = Model::default();

    let mut cmd = app.update(Event::Exists, &mut model);

    cmd.expect_no_events();
    let mut request = cmd.expect_one_effect().expect_exists();

    assert_eq!(
        request.operation,
        operation::Exists {
            key: "test".to_string()
        }
    );

    request
        .resolve(BoolResult::Ok(true))
        .expect("effect should resolve");

    let event = cmd.expect_one_event();
    app.update(event, &mut model)
        .expect_one_effect()
        .expect_render();

    assert!(model.successful);
}

#[test]
fn test_list_keys() {
    let app = App;
    let mut model = Model::default();

    let mut cmd = app.update(Event::ListKeys, &mut model);

    cmd.expect_no_events();
    let mut request = cmd.expect_one_effect().expect_list_keys();

    assert_eq!(
        request.operation,
        operation::ListKeys {
            prefix: "test:".to_string(),
            cursor: 0,
        }
    );

    request
        .resolve(KeysResult::Ok(Keys {
            keys: vec!["test:1".to_string(), "test:2".to_string()],
            next_cursor: 2,
        }))
        .expect("effect should resolve");

    let event = cmd.expect_one_event();
    app.update(event, &mut model)
        .expect_one_effect()
        .expect_render();

    assert_eq!(model.keys, vec!["test:1".to_string(), "test:2".to_string()]);
    assert_eq!(model.cursor, 2);
}

#[test]
fn test_kv_async() {
    let app = App;
    let mut model = Model::default();

    let mut cmd = app.update(Event::GetThenSet, &mut model);

    cmd.expect_no_events();
    let mut request = cmd.expect_one_effect().expect_get();

    assert_eq!(
        request.operation,
        operation::Get {
            key: "test_num".to_string()
        }
    );

    request
        .resolve(ValueResult::Ok(17u32.to_ne_bytes().to_vec().into()))
        .expect("effect should resolve");

    let mut request = cmd.expect_one_effect().expect_set();

    assert_eq!(
        request.operation,
        operation::Set {
            key: "test_num".to_string(),
            value: 18u32.to_ne_bytes().to_vec(),
        }
    );

    request
        .resolve(ValueResult::Ok(Value::None))
        .expect("effect should resolve");

    let event = cmd.expect_one_event();
    app.update(event, &mut model)
        .expect_one_effect()
        .expect_render();

    assert!(model.successful);
}

#[test]
fn test_error_is_passed_to_the_app() {
    let app = App;
    let mut model = Model::default();

    let mut cmd = app.update(Event::Exists, &mut model);
    let mut request = cmd.expect_one_effect().expect_exists();

    request
        .resolve(BoolResult::Err(KeyValueError::Timeout))
        .expect("effect should resolve");

    let event = cmd.expect_one_event();

    assert!(matches!(
        event,
        Event::ExistsResponse(Err(KeyValueError::Timeout))
    ));
}

#[test]
fn value_result_round_trips_through_data_result() {
    let cases = [
        ValueResult::Ok(Value::None),
        ValueResult::Ok(Value::Bytes(vec![1, 2, 3])),
        ValueResult::Err(KeyValueError::Timeout),
    ];

    for case in cases {
        let data: DataResult = case.clone().into();
        assert_eq!(ValueResult::from(data), case);
    }

    assert_eq!(
        DataResult::from(ValueResult::Ok(Value::Bytes(vec![1, 2, 3]))),
        Ok(Some(vec![1, 2, 3]))
    );
    // `Value::None` and an empty byte string are distinct on the wire, but
    // both `Ok(None)` and `Ok(Some(vec![]))` map back onto them faithfully.
    assert_eq!(DataResult::from(ValueResult::Ok(Value::None)), Ok(None));
}

#[test]
fn bool_result_round_trips_through_status_result() {
    let cases = [
        BoolResult::Ok(true),
        BoolResult::Ok(false),
        BoolResult::Err(KeyValueError::Io {
            message: "nope".to_string(),
        }),
    ];

    for case in cases {
        let status: StatusResult = case.clone().into();
        assert_eq!(BoolResult::from(status), case);
    }
}

#[test]
fn keys_result_round_trips_through_list_result() {
    let cases = [
        KeysResult::Ok(Keys {
            keys: vec!["a".to_string()],
            next_cursor: 7,
        }),
        KeysResult::Err(KeyValueError::CursorNotFound),
    ];

    for case in cases {
        let list: ListResult = case.clone().into();
        assert_eq!(KeysResult::from(list), case);
    }

    assert_eq!(
        ListResult::from(KeysResult::Ok(Keys {
            keys: vec!["a".to_string()],
            next_cursor: 7,
        })),
        Ok((vec!["a".to_string()], 7))
    );
}

#[test]
fn test_set_debug_repr() {
    {
        // small
        let op = operation::Set {
            key: "my key".into(),
            value: b"my value".to_vec(),
        };
        let repr = format!("{op:?}");
        assert_eq!(repr, r#"Set { key: "my key", value: "my value" }"#);
    }

    {
        // big
        let op = operation::Set {
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
        // binary
        let op = operation::Set {
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

#[test]
fn test_serializing_the_operations_as_json() {
    let get = operation::Get {
        key: "key".to_string(),
    };
    assert_eq!(serde_json::to_string(&get).unwrap(), r#"{"key":"key"}"#);

    let set = operation::Set {
        key: "key".to_string(),
        value: vec![1, 2],
    };
    assert_eq!(
        serde_json::to_string(&set).unwrap(),
        r#"{"key":"key","value":[1,2]}"#
    );

    let list_keys = operation::ListKeys {
        prefix: "key".to_string(),
        cursor: 0,
    };
    assert_eq!(
        serde_json::to_string(&list_keys).unwrap(),
        r#"{"prefix":"key","cursor":0}"#
    );
}

#[test]
fn test_serializing_the_outputs_as_json() {
    assert_eq!(
        serde_json::to_string(&ValueResult::Ok(Value::Bytes(vec![1, 2]))).unwrap(),
        r#"{"Ok":{"Bytes":[1,2]}}"#
    );

    assert_eq!(
        serde_json::to_string(&ValueResult::Err(KeyValueError::CursorNotFound)).unwrap(),
        r#"{"Err":"cursorNotFound"}"#
    );

    assert_eq!(
        serde_json::to_string(&BoolResult::Ok(true)).unwrap(),
        r#"{"Ok":true}"#
    );

    assert_eq!(
        serde_json::to_string(&KeysResult::Ok(Keys {
            keys: vec!["a".to_string()],
            next_cursor: 1,
        }))
        .unwrap(),
        r#"{"Ok":{"keys":["a"],"next_cursor":1}}"#
    );
}
