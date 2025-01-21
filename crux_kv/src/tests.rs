use anyhow::Result;
use crux_core::{macros::Effect, render::Render, testing::AppTester, Command};
use serde::{Deserialize, Serialize};

use crate::{
    error::KeyValueError, value::Value, KeyValue, KeyValueOperation, KeyValueResponse,
    KeyValueResult,
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

    GetResponse(Result<Option<Vec<u8>>, KeyValueError>),
    SetResponse(Result<Option<Vec<u8>>, KeyValueError>),
    ExistsResponse(Result<bool, KeyValueError>),
    ListKeysResponse(Result<(Vec<String>, u64), KeyValueError>),
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

    type Capabilities = Capabilities;
    type Effect = Effect;

    fn update(
        &self,
        event: Event,
        model: &mut Model,
        caps: &Capabilities,
    ) -> Command<Effect, Event> {
        let key = "test".to_string();
        match event {
            Event::Get => caps.key_value.get(key, Event::GetResponse),
            Event::Set => {
                caps.key_value
                    .set(key, 42i32.to_ne_bytes().to_vec(), Event::SetResponse);
            }
            Event::Delete => caps.key_value.delete(key, Event::SetResponse),
            Event::Exists => caps.key_value.exists(key, Event::ExistsResponse),
            Event::ListKeys => {
                caps.key_value
                    .list_keys("test:".to_string(), 0, Event::ListKeysResponse)
            }

            Event::GetThenSet => caps.compose.spawn(|ctx| {
                let kv = caps.key_value.clone();

                async move {
                    let Result::Ok(Some(value)) = kv.get_async("test_num".to_string()).await else {
                        panic!("expected get response with a value");
                    };

                    let num = i32::from_ne_bytes(value.try_into().unwrap());
                    let result = kv
                        .set_async("test_num".to_string(), (num + 1).to_ne_bytes().to_vec())
                        .await;

                    ctx.update_app(Event::SetResponse(result))
                }
            }),

            Event::GetResponse(Ok(Some(value))) => {
                let (int_bytes, _rest) = value.split_at(std::mem::size_of::<i32>());
                model.value = i32::from_ne_bytes(int_bytes.try_into().unwrap());
            }

            Event::GetResponse(Ok(None)) => {
                panic!("expected value");
            }

            Event::SetResponse(Ok(_response)) => {
                model.successful = true;
                caps.render.render()
            }

            Event::ExistsResponse(Ok(_response)) => {
                model.successful = true;
                caps.render.render()
            }

            Event::ListKeysResponse(Ok((keys, cursor))) => {
                model.keys = keys;
                model.cursor = cursor;
                caps.render.render()
            }

            Event::GetResponse(Err(error)) => {
                panic!("error: {:?}", error);
            }
            Event::SetResponse(Err(error)) => {
                panic!("error: {:?}", error);
            }
            Event::ExistsResponse(Err(error)) => {
                panic!("Error: {:?}", error);
            }
            Event::ListKeysResponse(Err(error)) => {
                panic!("Error: {:?}", error);
            }
        }

        Command::done()
    }

    fn view(&self, model: &Self::Model) -> Self::ViewModel {
        ViewModel {
            result: format!("Success: {}, Value: {}", model.successful, model.value),
        }
    }
}

#[derive(Effect)]
pub struct Capabilities {
    pub key_value: KeyValue<Event>,
    pub render: Render<Event>,
    #[effect(skip)]
    pub compose: crux_core::compose::Compose<Event>,
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
        KeyValueResult::Ok {
            response: KeyValueResponse::Get {
                value: 42i32.to_ne_bytes().to_vec().into(),
            },
        },
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
        KeyValueResult::Ok {
            response: KeyValueResponse::Set {
                previous: Value::None,
            },
        },
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
        KeyValueResult::Ok {
            response: KeyValueResponse::Delete {
                previous: Value::None,
            },
        },
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
        KeyValueResult::Ok {
            response: KeyValueResponse::Exists { is_present: true },
        },
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
        KeyValueResult::Ok {
            response: KeyValueResponse::ListKeys {
                keys: vec!["test:1".to_string(), "test:2".to_string()],
                next_cursor: 2,
            },
        },
        &mut model,
    );

    assert_eq!(model.keys, vec!["test:1".to_string(), "test:2".to_string()]);
    assert_eq!(model.cursor, 2);
}

#[test]
pub fn test_kv_async() -> Result<()> {
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
            KeyValueResult::Ok {
                response: KeyValueResponse::Get {
                    value: 17u32.to_ne_bytes().to_vec().into(),
                },
            },
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
        KeyValueResult::Ok {
            response: KeyValueResponse::Set {
                previous: Value::None,
            },
        },
        &mut model,
    );

    assert!(model.successful);

    Ok(())
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
