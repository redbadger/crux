use anyhow::{bail, Result};
use async_std::{
    fs::{File, OpenOptions},
    io::{ReadExt, WriteExt},
};
use chrono::{DateTime, Utc};
use clap::Parser;
use shared::{
    http::protocol::{HttpRequest, HttpResponse},
    key_value::{KeyValueOperation, KeyValueOutput},
    platform::PlatformResponse,
    time::TimeResponse,
    CatFactCapabilities, CatFacts, Core, Effect, Event,
};
use std::{collections::VecDeque, time::SystemTime};

enum Task {
    Event(Event),
    Effect(Effect),
}

#[derive(Parser, Clone)]
enum Command {
    Clear,
    Get,
    Fetch,
}

/// Simple program to greet a person
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    cmd: Command,
}

#[async_std::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let core: Core<Effect, CatFacts> = Core::new::<CatFactCapabilities>();

    let mut queue: VecDeque<Task> = VecDeque::new();

    queue.push_back(Task::Event(Event::Restore));
    queue.push_back(Task::Event(Event::GetPlatform));

    while !queue.is_empty() {
        let task = queue.pop_front().expect("an event");

        match task {
            Task::Event(event) => {
                enqueue_effects(&mut queue, core.process_event(event));
            }
            Task::Effect(effect) => match effect {
                Effect::Render(_) => (),
                Effect::Time(mut request) => {
                    let now: DateTime<Utc> = SystemTime::now().into();
                    let iso_time = now.to_rfc3339();
                    let response = TimeResponse(iso_time);

                    enqueue_effects(&mut queue, core.resolve(&mut request, response));
                }
                Effect::Http(mut request) => {
                    let HttpRequest { ref url, .. } = request.operation;
                    match surf::get(url).recv_bytes().await {
                        Ok(bytes) => {
                            let response = HttpResponse {
                                status: 200,
                                body: bytes,
                            };

                            enqueue_effects(&mut queue, core.resolve(&mut request, response));
                        }
                        Err(e) => bail!("Could not HTTP GET from {}: {}", &url, e),
                    }
                }
                Effect::Platform(mut request) => {
                    let response = PlatformResponse("cli".to_string());
                    enqueue_effects(&mut queue, core.resolve(&mut request, response));
                }
                Effect::KeyValue(mut request) => match request.operation {
                    KeyValueOperation::Read(ref key) => {
                        let bytes = read_state(key).await.ok();
                        let response = KeyValueOutput::Read(bytes);

                        let initial_msg = match &args.cmd {
                            Command::Clear => Task::Event(Event::Clear),
                            Command::Get => Task::Event(Event::Get),
                            Command::Fetch => Task::Event(Event::Fetch),
                        };

                        queue.push_back(initial_msg);
                        enqueue_effects(&mut queue, core.resolve(&mut request, response));
                    }
                    KeyValueOperation::Write(ref key, ref value) => {
                        let success = write_state(key, value).await.is_ok();
                        let response = KeyValueOutput::Write(success);

                        enqueue_effects(&mut queue, core.resolve(&mut request, response));
                    }
                },
            },
        }
    }

    let view = core.view();
    println!("platform: {}", view.platform);
    println!("{}", view.fact);

    Ok(())
}

fn enqueue_effects(queue: &mut VecDeque<Task>, effects: Vec<Effect>) {
    queue.append(&mut effects.into_iter().map(Task::Effect).collect())
}

async fn write_state(_key: &str, bytes: &[u8]) -> Result<()> {
    let mut f = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(".cat_facts")
        .await?;

    f.write_all(bytes).await?;

    Ok(())
}

async fn read_state(_key: &str) -> Result<Vec<u8>> {
    let mut f = File::open(".cat_facts").await?;
    let mut buf: Vec<u8> = vec![];

    f.read_to_end(&mut buf).await?;

    Ok(buf)
}
