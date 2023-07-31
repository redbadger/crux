mod http;
mod sse;

use std::sync::{Arc, Weak};

use async_std::task::spawn;
use clap::Parser;
use crossbeam_channel::{unbounded, Sender};
use eyre::{ErrReport, Result};
use futures::TryStreamExt;

use shared::{App, Capabilities, Core, Effect, Event};

#[derive(Debug)]
enum Message {
    Event(Event),
    Effect(Effect),
}

#[derive(Parser, Clone)]
enum Command {
    Get,
    Inc,
    Dec,
    Watch,
}

impl From<Command> for Message {
    fn from(cmd: Command) -> Self {
        match cmd {
            Command::Get => Message::Event(Event::Get),
            Command::Inc => Message::Event(Event::Increment),
            Command::Dec => Message::Event(Event::Decrement),
            Command::Watch => Message::Event(Event::StartWatch),
        }
    }
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    cmd: Command,
}

fn main() -> Result<()> {
    let (tx, rx) = unbounded::<Message>();

    let strong_tx = Arc::new(tx);
    let tx = Arc::downgrade(&strong_tx);

    let core = Arc::new(Core::new::<Capabilities>());

    // Kick off with the given command
    main_loop(&core, Args::parse().cmd.into(), tx.clone())?;
    drop(strong_tx); // tx may still live in a side-effect futures

    // Continue until there's no more work to do
    while let Ok(msg) = rx.recv() {
        main_loop(&core, msg, tx.clone())?;
    }

    Ok(())
}

fn main_loop(
    core: &Arc<Core<Effect, App>>,
    message: Message,
    tx: Weak<Sender<Message>>,
) -> Result<(), ErrReport> {
    match message {
        Message::Event(event) => {
            for effect in core.process_event(event) {
                process_effect(effect, core, tx.clone())?
            }
        }
        Message::Effect(effect) => process_effect(effect, core, tx)?,
    }

    Ok(())
}

fn process_effect(
    effect: Effect,
    core: &Arc<Core<Effect, App>>,
    tx: Weak<Sender<Message>>,
) -> Result<(), ErrReport> {
    match effect {
        Effect::Render(_) => {
            let view = core.view();

            if view.confirmed {
                println!("{text}", text = view.text);
            }
        }
        Effect::Http(mut request) => {
            spawn({
                let core = core.clone();
                let tx = tx.upgrade().expect("Should be able to upgrade Weak tx");

                async move {
                    let response = http::request(&request.operation).await.unwrap();
                    for effect in core.resolve(&mut request, response) {
                        tx.send(Message::Effect(effect)).unwrap();
                    }
                }
            });
        }
        Effect::ServerSentEvents(mut request) => {
            spawn({
                let core = core.clone();
                let tx = tx.upgrade().unwrap();

                async move {
                    let mut stream = sse::request(&request.operation).await.unwrap();

                    while let Ok(Some(response)) = stream.try_next().await {
                        for effect in core.resolve(&mut request, response) {
                            tx.send(Message::Effect(effect)).unwrap();
                        }
                    }
                }
            });
        }
    };

    Ok(())
}
