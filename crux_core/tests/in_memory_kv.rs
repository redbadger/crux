mod app {
    use crux_core::{render::render, Command};
    use crux_kv::{command::KeyValue, error::KeyValueError};
    use crux_macros::Effect;
    use serde::Serialize;

    use crate::middlware::EffectWithKV;

    #[derive(Default)]
    pub struct TestApp;

    #[derive(Serialize)]
    pub enum Event {
        Set {
            key: String,
            value: String,
        },
        Show {
            key: String,
        },

        #[serde(skip)]
        ValueSet,
        ValueReceived(String, Result<Option<Vec<u8>>, KeyValueError>),
    }

    #[derive(Default, Clone, Serialize)]
    pub struct Model {
        pub current_key: Option<String>,
        pub current_value: Option<String>,
    }

    #[derive(Effect)]
    #[allow(unused)]
    pub struct Capabilities {
        kv: crux_kv::KeyValue<Event>,
        render: crux_core::render::Render<Event>,
    }

    // Allow the middleware to unpick the Effect
    impl EffectWithKV for Effect {
        fn into_kv(self) -> Option<crux_core::Request<crux_kv::KeyValueOperation>> {
            self.into_kv()
        }

        fn is_kv(&self) -> bool {
            self.is_kv()
        }
    }

    impl crux_core::App for TestApp {
        type Event = Event;

        type Model = Model;

        type ViewModel = Model;

        type Capabilities = Capabilities;

        type Effect = Effect;

        fn update(
            &self,
            event: Self::Event,
            model: &mut Self::Model,
            _caps: &Self::Capabilities,
        ) -> crux_core::Command<Effect, Event> {
            match event {
                Event::Set { key, value } => {
                    KeyValue::set(key, value.into()).then_send(|_| Event::ValueSet)
                }
                Event::Show { key } => {
                    KeyValue::get(key.clone()).then_send(|r| Event::ValueReceived(key, r))
                }
                Event::ValueSet => Command::done(),
                Event::ValueReceived(key, Ok(Some(bytes))) => {
                    if let Ok(value) = String::from_utf8(bytes) {
                        model.current_key = Some(key);
                        model.current_value = Some(value);
                    }

                    render()
                }
                Event::ValueReceived(_key, Err(_)) | Event::ValueReceived(_key, Ok(None)) => {
                    model.current_key = None;
                    model.current_value = None;

                    render()
                }
            }
        }

        fn view(&self, model: &Self::Model) -> Self::ViewModel {
            model.clone()
        }
    }
}

mod middlware {
    use std::{
        collections::{HashMap, VecDeque},
        sync::{Arc, Mutex},
    };

    use crux_core::{App, Middleware, Request};
    use crux_kv::{
        error::KeyValueError, value::Value, KeyValueOperation, KeyValueResponse, KeyValueResult,
    };

    #[derive(Default, Clone)]
    struct Storage(Arc<Mutex<HashMap<String, Vec<u8>>>>);

    impl Storage {
        fn process_kv_operation(&self, operation: KeyValueOperation) -> KeyValueResult {
            match operation {
                KeyValueOperation::Get { key } => match self.0.lock() {
                    Err(_) => KeyValueResult::Err {
                        error: KeyValueError::Io {
                            message: "Could not acquire KV store lock".to_string(),
                        },
                    },
                    Ok(store) => match store.get(&key) {
                        Some(value) => KeyValueResult::Ok {
                            response: KeyValueResponse::Get {
                                value: Value::Bytes(value.clone()),
                            },
                        },
                        None => KeyValueResult::Ok {
                            response: KeyValueResponse::Get { value: Value::None },
                        },
                    },
                },
                KeyValueOperation::Set { key, value } => match self.0.lock() {
                    Err(_) => KeyValueResult::Err {
                        error: KeyValueError::Io {
                            message: "Could not acquire KV store lock".to_string(),
                        },
                    },
                    Ok(mut store) => match store.insert(key, value) {
                        Some(previous_value) => KeyValueResult::Ok {
                            response: KeyValueResponse::Set {
                                previous: Value::Bytes(previous_value),
                            },
                        },
                        None => KeyValueResult::Ok {
                            response: KeyValueResponse::Set {
                                previous: Value::None,
                            },
                        },
                    },
                },

                // Skip the rest, this is a demo implementation
                _ => unimplemented!(),
            }
        }
    }

    pub struct InMemoryKv<Core: crux_core::Middleware> {
        store: Storage,
        core: Core,
    }

    pub trait EffectWithKV {
        fn is_kv(&self) -> bool;

        fn into_kv(self) -> Option<Request<KeyValueOperation>>;
    }

    // This is a special iterator which skips and queues up KV effects, until the previous iterator runs out,
    // then pulls of the queue and processes the KV effect, and reads the new iterator in the same way
    struct QueueKV<'core, Core>
    where
        Core: Middleware,
    {
        kv_requests: VecDeque<Request<KeyValueOperation>>,
        inner: Box<dyn Iterator<Item = <<Core as Middleware>::App as App>::Effect> + 'core>,
        store: Storage,
        core: &'core Core,
    }

    impl<'core, Core> Iterator for QueueKV<'core, Core>
    where
        Core: Middleware,
        <<Core as Middleware>::App as App>::Effect: EffectWithKV,
    {
        type Item = <<Core as Middleware>::App as App>::Effect;

        fn next(&mut self) -> Option<Self::Item> {
            loop {
                while let Some(effect) = self.inner.next() {
                    if !effect.is_kv() {
                        // Found a non-KV effect return it
                        return Some(effect);
                    }

                    let Some(kv_request) = effect.into_kv() else {
                        unreachable!();
                    };

                    // collect the KV effect and continue iterating
                    self.kv_requests.push_back(kv_request);
                }

                let Some(mut kv_request) = self.kv_requests.pop_front() else {
                    // No more KV requests, we're finished
                    return None;
                };

                // process the next KV request and swap the inner iterator
                // for the iterator over follow-up effects

                let mut operation = KeyValueOperation::Get {
                    key: "".to_string(),
                };
                std::mem::swap(&mut kv_request.operation, &mut operation);

                let result = self.store.process_kv_operation(operation);

                kv_request.resolve(result).expect("to resolve");

                let follow_ups = self.core.process_effects();

                self.inner = Box::new(follow_ups);
                // Now we go back up to the while let to process the new iterator
            }
        }
    }

    impl<Core: Middleware> InMemoryKv<Core>
    where
        <<Core as crux_core::Middleware>::App as crux_core::App>::Effect: EffectWithKV,
    {
        pub fn new(core: Core) -> Self {
            Self {
                store: Default::default(),
                core,
            }
        }

        fn process_kv<'a>(
            &'a self,
            effects: impl Iterator<Item = <<Core as Middleware>::App as App>::Effect> + 'a,
        ) -> QueueKV<'a, Core>
        where
            <Core::App as crux_core::App>::Effect: EffectWithKV,
        {
            QueueKV {
                kv_requests: VecDeque::new(),
                inner: Box::new(effects),
                store: self.store.clone(),
                core: &self.core,
            }
        }
    }

    impl<Core: Middleware> crux_core::Middleware for InMemoryKv<Core>
    where
        <Core::App as crux_core::App>::Effect: EffectWithKV,
    {
        type App = <Core as Middleware>::App;

        fn process_event(
            &self,
            event: <Self::App as crux_core::App>::Event,
        ) -> impl Iterator<Item = <Self::App as crux_core::App>::Effect> {
            self.process_kv(self.core.process_event(event))
        }

        fn process_effects(&self) -> impl Iterator<Item = <Self::App as crux_core::App>::Effect> {
            self.process_kv(self.core.process_effects())
        }

        fn view(&self) -> <Self::App as crux_core::App>::ViewModel {
            self.core.view()
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        app::{Effect, Event, TestApp},
        middlware::InMemoryKv,
    };
    use crux_core::{Core, Middleware};

    #[test]
    fn kv_gets_processed() {
        let core = InMemoryKv::new(Core::<TestApp>::new());

        // Nothing shown

        let view = core.view();
        assert_eq!(view.current_key, None);
        assert_eq!(view.current_value, None);

        let mut effects = core.process_event(Event::Set {
            key: "captain".to_string(),
            value: "Jean-Luc Picard".to_string(),
        });

        let effect = effects.next();
        assert!(effect.is_none());

        // Still nothing shown

        let view = core.view();
        assert_eq!(view.current_key, None);
        assert_eq!(view.current_value, None);

        let mut effects = core.process_event(Event::Set {
            key: "first-officer".to_string(),
            value: "William T. Riker".to_string(),
        });

        assert!(effects.next().is_none());

        // Still nothing shown

        let view = core.view();
        assert_eq!(view.current_key, None);
        assert_eq!(view.current_value, None);

        let mut effects = core.process_event(Event::Show {
            key: "captain".to_string(),
        });

        assert!(matches!(effects.next(), Some(Effect::Render(_))));

        // Correct value shown, KV must have worked!

        let view = core.view();
        assert_eq!(view.current_key, Some("captain".to_string()));
        assert_eq!(view.current_value, Some("Jean-Luc Picard".to_string()));
    }
}
