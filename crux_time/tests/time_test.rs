#[cfg(nope)]
mod shared {
    use crux_core::{render::Render, App, Capabilities, Command, Commander};
    use crux_time::{Time, TimeResponse};
    use serde::{Deserialize, Serialize};
    use std::marker::PhantomData;

    #[derive(Default)]
    pub struct MyApp<Ef, Caps> {
        _marker: PhantomData<fn() -> (Ef, Caps)>,
    }

    #[derive(Serialize, Deserialize)]
    pub enum MyEvent {
        TimeGet,
        TimeSet(TimeResponse),
    }

    #[derive(Default, Serialize, Deserialize)]
    pub struct MyModel {
        pub time: String,
    }

    #[derive(Serialize, Deserialize, Default)]
    pub struct MyViewModel {
        pub time: String,
    }

    impl<Ef, Caps> App<Ef, Caps> for MyApp<Ef, Caps>
    where
        Ef: Serialize + Clone + Default,
        Caps: Default + Capabilities<Time<Ef>> + Capabilities<Render<Ef>>,
    {
        type Event = MyEvent;
        type Model = MyModel;
        type ViewModel = MyViewModel;

        fn update(
            &self,
            event: MyEvent,
            model: &mut MyModel,
            caps: &Caps,
            commander: &Commander<Command<Ef, Self::Event>>,
        ) {
            let time: &Time<_> = caps.get();
            let render: &Render<_> = caps.get();

            match event {
                MyEvent::TimeGet => commander.send_command(time.get(MyEvent::TimeSet)),
                MyEvent::TimeSet(time) => {
                    model.time = time.0;
                    commander.send_command(render.render())
                }
            }
        }

        fn view(
            &self,
            model: &<Self as App<Ef, Caps>>::Model,
        ) -> <Self as App<Ef, Caps>>::ViewModel {
            MyViewModel {
                time: model.time.clone(),
            }
        }
    }

    #[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
    pub enum MyEffect {
        Time,
        Render,
    }

    impl Default for MyEffect {
        fn default() -> Self {
            MyEffect::Render
        }
    }

    pub(crate) struct MyCapabilities {
        pub time: Time<MyEffect>,
        pub render: Render<MyEffect>,
    }

    impl crux_core::Capabilities<Time<MyEffect>> for MyCapabilities {
        fn get(&self) -> &Time<MyEffect> {
            &self.time
        }
    }

    impl crux_core::Capabilities<Render<MyEffect>> for MyCapabilities {
        fn get(&self) -> &Render<MyEffect> {
            &self.render
        }
    }

    impl Default for MyCapabilities {
        fn default() -> Self {
            Self {
                time: Time::new(MyEffect::Time),
                render: Render::new(MyEffect::Render),
            }
        }
    }
}

#[cfg(nope)]
mod shell {
    use super::shared::{MyApp, MyCapabilities, MyEffect, MyEvent, MyViewModel};
    use anyhow::Result;
    use crux_core::{Core, Request};
    use crux_time::TimeResponse;
    use std::collections::VecDeque;

    pub enum Outcome {
        Time(TimeResponse),
    }

    enum CoreMessage {
        Message(MyEvent),
        Response(Vec<u8>, Outcome),
    }

    pub fn run() -> Result<(Vec<MyEffect>, MyViewModel)> {
        let core: Core<MyEffect, MyCapabilities, MyApp<MyEffect, MyCapabilities>> = Core::new();
        let mut queue: VecDeque<CoreMessage> = VecDeque::new();

        queue.push_back(CoreMessage::Message(MyEvent::TimeGet));

        let mut received = vec![];

        while !queue.is_empty() {
            let msg = queue.pop_front();

            let reqs = match msg {
                Some(CoreMessage::Message(m)) => core.message(&bcs::to_bytes(&m)?),
                Some(CoreMessage::Response(uuid, output)) => core.response(
                    &uuid,
                    &match output {
                        Outcome::Time(x) => bcs::to_bytes(&x)?,
                    },
                ),
                _ => vec![],
            };
            let reqs: Vec<Request<MyEffect>> = bcs::from_bytes(&reqs)?;

            for req in reqs {
                let Request { uuid, effect } = req;
                match effect {
                    MyEffect::Render => received.push(effect),
                    MyEffect::Time => {
                        received.push(effect);
                        queue.push_back(CoreMessage::Response(
                            uuid,
                            Outcome::Time(TimeResponse(
                                "2022-12-01T01:47:12.746202562+00:00".to_string(),
                            )),
                        ));
                    }
                }
            }
        }

        let view = bcs::from_bytes::<MyViewModel>(&core.view())?;
        Ok((received, view))
    }
}

#[cfg(nope)]
mod tests {
    use crate::{shared::MyEffect, shell::run};
    use anyhow::Result;

    #[test]
    pub fn test_time() -> Result<()> {
        let (received, view) = run()?;
        assert_eq!(received, vec![MyEffect::Time, MyEffect::Render]);
        assert_eq!(view.time, "2022-12-01T01:47:12.746202562+00:00");
        Ok(())
    }
}
