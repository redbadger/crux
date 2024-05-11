use anyhow::Result;
use crux_core::{macros::Effect, render::Render, testing::AppTester};
use serde::{Deserialize, Serialize};

use crate::{KeyValue, KeyValueReadResult, KeyValueRequest, KeyValueResponse, KeyValueWriteResult};

#[derive(Default)]
pub struct App;

#[derive(Debug, Serialize, Deserialize)]
pub enum Event {
    Get,
    Set,
    Delete,
    Exists,
    GetThenSet,

    Response(KeyValueResponse),
}

#[derive(Debug, Default)]
pub struct Model {
    pub value: i32,
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

    fn update(&self, event: Event, model: &mut Model, caps: &Capabilities) {
        let key = "test".to_string();
        match event {
            Event::Get => caps.key_value.get(key, Event::Response),
            Event::Set => {
                caps.key_value
                    .set(key, 42i32.to_ne_bytes().to_vec(), Event::Response);
            }
            Event::Delete => caps.key_value.delete(key, Event::Response),
            Event::Exists => caps.key_value.exists(key, Event::Response),

            Event::GetThenSet => caps.compose.spawn(|ctx| {
                let kv = caps.key_value.clone();

                async move {
                    let KeyValueResponse::Get { result } =
                        kv.get_async("test_num".to_string()).await
                    else {
                        panic!("expected get response");
                    };

                    let KeyValueReadResult::Data { value } = result else {
                        panic!("Get failed;");
                    };

                    let num = i32::from_ne_bytes(value.try_into().unwrap());
                    let result = kv
                        .set_async("test_num".to_string(), (num + 1).to_ne_bytes().to_vec())
                        .await;

                    ctx.update_app(Event::Response(result))
                }
            }),

            Event::Response(KeyValueResponse::Get { result }) => {
                if let KeyValueReadResult::Data { value } = result {
                    let (int_bytes, _rest) = value.split_at(std::mem::size_of::<i32>());
                    model.value = i32::from_ne_bytes(int_bytes.try_into().unwrap());
                }
                caps.render.render()
            }
            Event::Response(KeyValueResponse::Set { result })
            | Event::Response(KeyValueResponse::Delete { result }) => {
                model.successful = matches!(result, KeyValueWriteResult::Ok { .. });
                caps.render.render()
            }
            Event::Response(KeyValueResponse::Exists { result }) => {
                model.successful = matches!(result, KeyValueReadResult::Exists { .. });
                caps.render.render()
            }
        }
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
    let app = AppTester::<App, _>::default();
    let mut model = Model::default();

    let updated = app.update(Event::Get, &mut model);

    let effect = updated.into_effects().next().unwrap();
    let Effect::KeyValue(mut request) = effect else {
        panic!("Expected KeyValue effect");
    };

    let KeyValueRequest::Get { key } = request.operation.clone() else {
        panic!("Expected get operation");
    };

    assert_eq!(key, "test");

    let updated = app
        .resolve(
            &mut request,
            KeyValueResponse::Get {
                result: KeyValueReadResult::Data {
                    value: 42i32.to_ne_bytes().to_vec(),
                },
            },
        )
        .unwrap();

    let event = updated.events.into_iter().next().unwrap();
    app.update(event, &mut model);

    assert_eq!(model.value, 42);
}

#[test]
fn test_set() {
    let app = AppTester::<App, _>::default();
    let mut model = Model::default();

    let updated = app.update(Event::Set, &mut model);

    let effect = updated.into_effects().next().unwrap();
    let Effect::KeyValue(mut request) = effect else {
        panic!("Expected KeyValue effect");
    };

    let KeyValueRequest::Set { key, value } = request.operation.clone() else {
        panic!("Expected set operation");
    };

    assert_eq!(key, "test");
    assert_eq!(value, 42i32.to_ne_bytes().to_vec());

    let updated = app
        .resolve(
            &mut request,
            KeyValueResponse::Set {
                result: KeyValueWriteResult::Ok { previous: vec![] },
            },
        )
        .unwrap();

    let event = updated.events.into_iter().next().unwrap();
    app.update(event, &mut model);

    assert!(model.successful);
}

#[test]
fn test_delete() {
    let app = AppTester::<App, _>::default();
    let mut model = Model::default();

    let updated = app.update(Event::Delete, &mut model);

    let effect = updated.into_effects().next().unwrap();
    let Effect::KeyValue(mut request) = effect else {
        panic!("Expected KeyValue effect");
    };

    let KeyValueRequest::Delete { key } = request.operation.clone() else {
        panic!("Expected delete operation");
    };

    assert_eq!(key, "test");

    let updated = app
        .resolve(
            &mut request,
            KeyValueResponse::Delete {
                result: KeyValueWriteResult::Ok { previous: vec![] },
            },
        )
        .unwrap();

    let event = updated.events.into_iter().next().unwrap();
    app.update(event, &mut model);

    assert!(model.successful);
}

#[test]
fn test_exists() {
    let app = AppTester::<App, _>::default();
    let mut model = Model::default();

    let updated = app.update(Event::Exists, &mut model);

    let effect = updated.into_effects().next().unwrap();
    let Effect::KeyValue(mut request) = effect else {
        panic!("Expected KeyValue effect");
    };

    let KeyValueRequest::Exists { key } = request.operation.clone() else {
        panic!("Expected exists operation");
    };

    assert_eq!(key, "test");

    let updated = app
        .resolve(
            &mut request,
            KeyValueResponse::Exists {
                result: KeyValueReadResult::Exists { value: true },
            },
        )
        .unwrap();

    let event = updated.events.into_iter().next().unwrap();
    app.update(event, &mut model);

    assert!(model.successful);
}

#[test]
pub fn test_kv_async() -> Result<()> {
    let app = AppTester::<App, _>::default();
    let mut model = Model::default();

    let update = app.update(Event::GetThenSet, &mut model);

    let effect = update.into_effects().next().unwrap();
    let Effect::KeyValue(mut request) = effect else {
        panic!("Expected KeyValue effect");
    };

    let KeyValueRequest::Get { key } = request.operation.clone() else {
        panic!("Expected get operation");
    };

    assert_eq!(key, "test_num");

    let update = app
        .resolve(
            &mut request,
            KeyValueResponse::Get {
                result: KeyValueReadResult::Data {
                    value: 17u32.to_ne_bytes().to_vec(),
                },
            },
        )
        .unwrap();

    let effect = update.into_effects().next().unwrap();
    let Effect::KeyValue(mut request) = effect else {
        panic!("Expected KeyValue effect");
    };

    let KeyValueRequest::Set { key, value } = request.operation.clone() else {
        panic!("Expected get operation");
    };

    assert_eq!(key, "test_num".to_string());
    assert_eq!(value, 18u32.to_ne_bytes().to_vec());

    let update = app
        .resolve(
            &mut request,
            KeyValueResponse::Set {
                result: KeyValueWriteResult::Ok { previous: vec![] },
            },
        )
        .unwrap();

    let event = update.events.into_iter().next().unwrap();
    app.update(event, &mut model);

    assert!(model.successful);

    Ok(())
}
