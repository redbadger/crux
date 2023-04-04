mod capability {
    use async_channel::Sender;
    use crux_core::{
        capability::{CapabilityContext, Operation},
        Capability,
    };
    use futures::StreamExt;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Fetch {
        pub id: usize,
    }

    impl Operation for Fetch {
        type Output = Vec<usize>; // links to other items
    }

    pub struct Crawler<Ev> {
        context: CapabilityContext<Fetch, Ev>,
        tasks_tx: Sender<(usize, Sender<usize>)>,
    }

    const NUM_WORKERS: usize = 3;

    // A hypothetical asynchronous crawler, which fetches remote items
    // and discovers links for further items in them. Imagine a web crawler.
    // To avoid fetching too many items concurrently it has a worker pool
    //
    // We use this in the test to make sure the capability runtime supports
    // this type of use-case correctly.
    impl<Ev> Crawler<Ev>
    where
        Ev: 'static,
    {
        pub fn new(context: CapabilityContext<Fetch, Ev>) -> Self {
            let (tasks_tx, tasks_rx) = async_channel::unbounded::<(usize, Sender<usize>)>();

            for n in 0..NUM_WORKERS {
                context.spawn({
                    let context = context.clone();
                    let tasks_rx = tasks_rx.clone();
                    let tasks_tx = tasks_tx.clone();

                    async move {
                        while let Ok((id, results_tx)) = tasks_rx.recv().await {
                            results_tx.send(id).await.unwrap();

                            println!("Worker {n} fetching #{id}");
                            let more_ids = context.request_from_shell(Fetch { id }).await;
                            for id in more_ids {
                                tasks_tx.send((id, results_tx.clone())).await.unwrap();
                            }
                        }
                    }
                });
            }

            Crawler { context, tasks_tx }
        }

        pub fn fetch_tree<F>(&self, id: usize, ev: F)
        where
            F: Fn(Vec<usize>) -> Ev + Send + 'static,
        {
            let (results_tx, results_rx) = async_channel::unbounded::<usize>();
            let tasks_tx = self.tasks_tx.clone();

            self.context.spawn({
                let context = self.context.clone();

                async move {
                    tasks_tx.send((id, results_tx)).await.unwrap();

                    let results: Vec<_> = results_rx.collect().await;

                    context.update_app(ev(results));
                }
            });
        }
    }

    impl<Ev> Capability<Ev> for Crawler<Ev> {
        type Operation = Fetch;
        type MappedSelf<MappedEv> = Crawler<MappedEv>;

        fn map_event<F, NewEvent>(&self, f: F) -> Self::MappedSelf<NewEvent>
        where
            F: Fn(NewEvent) -> Ev + Send + Sync + Copy + 'static,
            Ev: 'static,
            NewEvent: 'static,
        {
            Self::MappedSelf::new(self.context.map_event(f))
        }
    }
}

mod app {
    use crux_core::App;
    use crux_macros::Effect;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    pub enum Event {
        Fetch,
        Done(Vec<usize>),
    }

    #[derive(Effect)]
    #[effect(app = "MyApp")]
    pub struct Capabilities {
        crawler: super::capability::Crawler<Event>,
        render: crux_core::render::Render<Event>,
    }

    #[derive(Default)]
    pub struct MyApp;

    impl App for MyApp {
        type Event = Event;
        type Model = Vec<usize>;
        type ViewModel = Vec<usize>;
        type Capabilities = Capabilities;

        fn update(&self, event: Self::Event, model: &mut Self::Model, caps: &Self::Capabilities) {
            match event {
                Event::Fetch => caps.crawler.fetch_tree(0, Event::Done),
                Event::Done(items) => {
                    *model = items;
                    caps.render.render();
                }
            }
        }

        fn view(&self, model: &Self::Model) -> Self::ViewModel {
            model.clone()
        }
    }
}

mod tests {
    use std::collections::VecDeque;

    use crux_core::Core;
    use rand::prelude::*;

    use super::app::{Capabilities, Effect, Event, MyApp};

    #[test]
    fn fetches_a_tree() {
        let core: Core<Effect, MyApp> = Core::new::<Capabilities>();

        let mut effects: VecDeque<Effect> = core.process_event(Event::Fetch).into();

        let mut counter: usize = 1;

        while !effects.is_empty() {
            let effect = effects.pop_front().unwrap();

            match effect {
                Effect::Crawler(mut request) => {
                    let output = if counter < 30 {
                        vec![counter, counter + 1, counter + 2]
                    } else {
                        vec![]
                    };

                    counter += 3;

                    let effs: Vec<Effect> = core.resolve(&mut request, output);

                    for e in effs {
                        effects.push_back(e)
                    }

                    // Simulate network timing
                    effects.make_contiguous().shuffle(&mut rand::thread_rng());
                }
                Effect::Render(_) => {
                    let view: Vec<usize> = core.view();
                    let expected: Vec<usize> = (0..=30).collect();

                    assert_eq!(view, expected);

                    return;
                }
            }
        }

        unreachable!("Capability never returned a result");
    }
}
