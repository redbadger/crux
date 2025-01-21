mod app {
    use crux_core::{macros::Effect, Command};
    use futures::future::join;
    use serde::Serialize;

    #[derive(Default, Clone)]
    pub struct App;

    #[derive(Debug, PartialEq)]
    pub enum Event {
        Trigger,
        Finished(usize, usize),
    }

    #[derive(Default, Serialize, Debug, PartialEq)]
    pub struct Model {
        pub total: usize,
    }

    #[derive(Effect)]
    pub struct Capabilities {
        one: super::capabilities::one::CapabilityOne<Event>,
        two: super::capabilities::two::CapabilityTwo<Event>,
        #[effect(skip)]
        compose: crux_core::compose::Compose<Event>,
    }

    impl crux_core::App for App {
        type Event = Event;
        type Model = Model;
        type ViewModel = Model;
        type Capabilities = Capabilities;
        type Effect = Effect;

        fn update(
            &self,
            event: Self::Event,
            model: &mut Self::Model,
            caps: &Self::Capabilities,
        ) -> Command<Effect, Event> {
            match event {
                Event::Trigger => caps.compose.spawn(|context| {
                    let one = caps.one.clone();
                    let two = caps.two.clone();

                    async move {
                        let (result_one, result_two) =
                            join(one.one_async(10), two.two_async(20)).await;

                        context.update_app(Event::Finished(result_one, result_two))
                    }
                }),
                Event::Finished(one, two) => {
                    model.total = one + two;
                }
            }

            Command::done()
        }

        fn view(&self, _model: &Self::Model) -> Self::ViewModel {
            todo!()
        }
    }
}

pub mod capabilities {
    pub mod one {
        use crux_core::capability::{CapabilityContext, Operation};
        use crux_core::macros::Capability;
        use serde::{Deserialize, Serialize};

        #[derive(PartialEq, Clone, Serialize, Deserialize, Debug)]
        pub struct OpOne {
            number: usize,
        }

        impl Operation for OpOne {
            type Output = usize;
        }

        #[derive(Capability)]
        pub struct CapabilityOne<E> {
            context: CapabilityContext<OpOne, E>,
        }

        // Needed to allow 'this = (*self).clone()' without requiring E: Clone
        // See https://github.com/rust-lang/rust/issues/26925
        impl<E> Clone for CapabilityOne<E> {
            fn clone(&self) -> Self {
                Self {
                    context: self.context.clone(),
                }
            }
        }

        impl<E> CapabilityOne<E> {
            pub fn new(context: CapabilityContext<OpOne, E>) -> Self {
                Self { context }
            }

            pub fn one<F>(&self, number: usize, event: F)
            where
                F: FnOnce(usize) -> E + Send + 'static,
                E: 'static,
            {
                let this = Clone::clone(self);

                this.context.spawn({
                    let this = this.clone();

                    async move {
                        let result = this.one_async(number).await;

                        this.context.update_app(event(result))
                    }
                });
            }

            pub async fn one_async(&self, number: usize) -> usize
            where
                E: 'static,
            {
                self.context.request_from_shell(OpOne { number }).await
            }
        }
    }

    pub mod two {
        use crux_core::capability::{CapabilityContext, Operation};
        use crux_core::macros::Capability;
        use serde::{Deserialize, Serialize};

        #[derive(PartialEq, Clone, Serialize, Deserialize, Debug)]
        pub struct OpTwo {
            number: usize,
        }

        impl Operation for OpTwo {
            type Output = usize;
        }

        #[derive(Capability)]
        pub struct CapabilityTwo<E> {
            context: CapabilityContext<OpTwo, E>,
        }

        // Needed to allow 'this = (*self).clone()' without requiring E: Clone
        // See https://github.com/rust-lang/rust/issues/26925
        impl<E> Clone for CapabilityTwo<E> {
            fn clone(&self) -> Self {
                Self {
                    context: self.context.clone(),
                }
            }
        }

        impl<E> CapabilityTwo<E> {
            pub fn new(context: CapabilityContext<OpTwo, E>) -> Self {
                Self { context }
            }

            pub fn two<F>(&self, number: usize, event: F)
            where
                F: FnOnce(usize) -> E + Send + 'static,
                E: 'static,
            {
                let this = Clone::clone(self);

                this.context.spawn({
                    let this = this.clone();

                    async move {
                        let result = this.two_async(number).await;

                        this.context.update_app(event(result))
                    }
                });
            }

            pub async fn two_async(&self, number: usize) -> usize
            where
                E: 'static,
            {
                self.context.request_from_shell(OpTwo { number }).await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crux_core::testing::AppTester;

    use crate::app::{Event, Model};

    use super::app::{App, Effect};

    #[test]
    fn updates_state_once_both_effects_are_done() {
        let app: AppTester<App> = AppTester::default();
        let mut model = Model::default();

        let update = app.update(Event::Trigger, &mut model);

        let mut effects = update.into_effects().filter(|e| e.is_one() || e.is_two());

        // Resolve the first effect
        // We should not see any events
        match effects.next().expect("there should be an effect") {
            Effect::CapabilityOne(mut req) => {
                let update = app.resolve(&mut req, 1).expect("should resolve");

                assert!(update.events.is_empty());
            }
            Effect::CapabilityTwo(mut req) => {
                let update = app.resolve(&mut req, 2).expect("should resolve");

                assert!(update.events.is_empty());
            }
        }

        // Resolve the second effect
        // This time we _should_ see an event
        let mut events = match effects.next().expect("there should be an effect") {
            Effect::CapabilityOne(mut req) => {
                let update = app.resolve(&mut req, 1).expect("should resolve");

                update.events
            }
            Effect::CapabilityTwo(mut req) => {
                let update = app.resolve(&mut req, 2).expect("should resolve");

                update.events
            }
        };

        assert_eq!(events, vec![Event::Finished(1, 2)]);
        let update = app.update(events.remove(0), &mut model);

        assert!(update.effects.is_empty());
        assert!(update.events.is_empty());

        assert_eq!(model, Model { total: 3 });
    }
}
