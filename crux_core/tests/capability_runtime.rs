mod capability {
    use std::future::Future;

    use async_channel::{Receiver, Sender};
    use crux_core::capability::{CapabilityContext, Operation};
    use crux_core::macros::Capability;
    use futures::future::BoxFuture;
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
    pub struct Crawler {
        context: CapabilityContext<Fetch>,
        tasks_tx: Sender<(usize, Sender<usize>)>,
        tasks_rx: Receiver<(usize, Sender<usize>)>,
        commands_tx: Sender<Command>,
        commands_rx: Receiver<Command>,
    }

    impl Drop for Crawler {
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
    impl Crawler {
        pub fn new(context: CapabilityContext<Fetch>) -> Self {
            let (tasks_tx, tasks_rx) = async_channel::unbounded::<(usize, Sender<usize>)>();
            let (commands_tx, commands_rx) = async_channel::unbounded::<Command>();
            Crawler {
                context,
                tasks_tx,
                tasks_rx,
                commands_tx,
                commands_rx,
            }
        }

        pub(crate) fn start_workers(&self) -> Vec<BoxFuture<'static, ()>> {
            (0..NUM_WORKERS).map(
                |n| {
                let context = self.context.clone();
                let tasks_rx = self.tasks_rx.clone();
                let tasks_tx = self.tasks_tx.clone();
                let commands_rx = self.commands_rx.clone();

                let mut accepting_tasks = true;

                let fut = async move {
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
                };
                Box::pin(fut) as BoxFuture<'static, ()>
            })
            .collect::<Vec<_>>()
        }

        pub fn pause(&self) -> impl Future<Output = ()> {
            let commands_tx = self.commands_tx.clone();
            async move {
                let _ = commands_tx.send(Command::Pause).await;
            }
        }

        pub fn resume(&self) -> impl Future<Output = ()> {
            let commands_tx = self.commands_tx.clone();
            async move {
                let _ = commands_tx.send(Command::Resume).await;
            }
        }

        pub fn fetch_tree(&self, id: usize) -> impl Future<Output = Vec<usize>> {
            let (results_tx, results_rx) = async_channel::unbounded::<usize>();
            let tasks_tx = self.tasks_tx.clone();

            async move {
                tasks_tx.send((id, results_tx)).await.unwrap();
                results_rx.collect().await
            }
        }
    }
}

mod app {
    use crux_core::App;
    use crux_core::{macros::Effect, Command};
    use futures::future::BoxFuture;
    use futures::FutureExt;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    pub enum Event {
        Start,
        Fetch,
        Pause,
        Resume,
        Done(Vec<usize>),
    }

    #[derive(Effect)]
    pub struct Capabilities {
        crawler: super::capability::Crawler,
        render: crux_core::render::Render,
    }

    #[derive(Default)]
    pub struct MyApp;

    impl App for MyApp {
        type Event = Event;
        type Model = Vec<usize>;
        type ViewModel = Vec<usize>;
        type Capabilities = Capabilities;

        fn update(
            &self,
            event: Self::Event,
            model: &mut Self::Model,
            caps: &Self::Capabilities,
        ) -> Command<Self::Event> {
            match event {
                Event::Start => Command::Effects(
                    caps.crawler
                        .start_workers()
                        .into_iter()
                        .map(|work| {
                            Box::pin(work.map(|()| Command::None))
                                as BoxFuture<'static, Command<Event>>
                        })
                        .collect(),
                ),
                Event::Fetch => {
                    let fut = caps.crawler.fetch_tree(0);
                    Command::effect(fut.map(|data| Command::Event(Event::Done(data))))
                }
                Event::Pause => Command::empty_effect(caps.crawler.pause()),
                Event::Resume => Command::empty_effect(caps.crawler.resume()),
                Event::Done(items) => {
                    *model = items;
                    caps.render.render()
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

    use super::app::{Effect, Event, MyApp};

    #[test]
    fn fetches_a_tree() {
        let core: Core<Effect, MyApp> = Core::new();

        let mut effects: VecDeque<_> = core
            .process_event(Event::Start)
            .into_iter()
            .chain(core.process_event(Event::Fetch))
            .collect();

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

    #[test]
    fn doesnt_crash_when_core_is_dropped() {
        let core: Core<Effect, MyApp> = Core::new();

        // Spawns the task
        core.process_event(Event::Fetch);

        drop(core);
    }
}
