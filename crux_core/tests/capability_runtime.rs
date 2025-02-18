mod capability {
    use async_channel::Sender;
    use crux_core::capability::{CapabilityContext, Operation};
    use crux_core::macros::Capability;
    use futures::{FutureExt, StreamExt};
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Fetch {
        pub id: usize,
    }

    impl Operation for Fetch {
        type Output = Vec<usize>; // links to other items
    }

    #[derive(Capability)]
    pub struct Crawler<Ev> {
        context: CapabilityContext<Fetch, Ev>,
        tasks_tx: Sender<(usize, Sender<usize>)>,
        commands_tx: Sender<Command>,
    }

    impl<Ev> Drop for Crawler<Ev> {
        fn drop(&mut self) {
            eprintln!("Dropping crawler");
        }
    }

    const NUM_WORKERS: usize = 3;

    enum Command {
        Pause,
        Resume,
    }

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
            let (commands_tx, commands_rx) = async_channel::unbounded::<Command>();

            for n in 0..NUM_WORKERS {
                context.spawn({
                    let context = context.clone();
                    let tasks_rx = tasks_rx.clone();
                    let tasks_tx = tasks_tx.clone();
                    let commands_rx = commands_rx.clone();

                    let mut accepting_tasks = true;

                    async move {
                        loop {
                            if accepting_tasks {
                                eprintln!("Worker {n} awaiting command or task");
                                futures::select! {
                                    command = commands_rx.recv().fuse() => {
                                        match command {
                                            Ok(Command::Pause) => {
                                                accepting_tasks = false
                                            },
                                            Ok(_) => {},
                                            Err(_) => break,
                                        }
                                    }
                                    task = tasks_rx.recv().fuse() => {
                                         match task {
                                             Ok((id, results_tx)) => {
                                                 results_tx.send(id).await.unwrap();

                                                 println!("Worker {n} fetching #{id}");
                                                 let more_ids = context.request_from_shell(Fetch { id }).await;
                                                 for id in more_ids {
                                                     tasks_tx.send((id, results_tx.clone())).await.unwrap();
                                                 }
                                             },
                                             Err(_) => break,
                                         }
                                    }
                                }
                            } else {
                                while let Ok(command) = commands_rx.recv().await {
                                    if let Command::Resume = command {
                                        accepting_tasks = true;

                                        break;
                                    }
                                }
                            }
                        }

                        eprintln!("Worker {n} stopping");
                    }
                });
            }

            Crawler {
                context,
                tasks_tx,
                commands_tx,
            }
        }

        pub fn pause(&self) {
            self.context.spawn({
                let commands_tx = self.commands_tx.clone();

                async move {
                    let _ = commands_tx.send(Command::Pause).await;
                }
            })
        }

        pub fn resume(&self) {
            self.context.spawn({
                let commands_tx = self.commands_tx.clone();

                async move {
                    let _ = commands_tx.send(Command::Resume).await;
                }
            })
        }

        pub fn fetch_tree<F>(&self, id: usize, ev: F)
        where
            F: FnOnce(Vec<usize>) -> Ev + Send + 'static,
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
}

mod app {
    use crux_core::App;
    use crux_core::{macros::Effect, Command};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    pub enum Event {
        Fetch,
        Pause,
        Resume,
        Done(Vec<usize>),
    }

    #[derive(Effect)]
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
        type Effect = Effect;

        fn update(
            &self,
            event: Self::Event,
            model: &mut Self::Model,
            caps: &Self::Capabilities,
        ) -> Command<Effect, Event> {
            match event {
                Event::Fetch => caps.crawler.fetch_tree(0, Event::Done),
                Event::Pause => caps.crawler.pause(),
                Event::Resume => caps.crawler.resume(),
                Event::Done(items) => {
                    *model = items;
                    caps.render.render();
                }
            }

            Command::done()
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

    use super::app::{Effect, Event, MyApp};

    #[test]
    fn fetches_a_tree() {
        let core: Core<MyApp> = Core::new();

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

                    let effs: Vec<Effect> = core.resolve(&mut request, output).expect("to resolve");

                    for e in effs {
                        effects.push_back(e)
                    }

                    // Simulate network timing
                    effects.make_contiguous().shuffle(&mut rand::rng());
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

    #[test]
    fn doesnt_crash_when_core_is_dropped() {
        let core: Core<MyApp> = Core::new();

        // Spawns the task
        core.process_event(Event::Fetch);

        drop(core);
    }
}
