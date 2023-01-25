use anyhow::{bail, Result};
use async_std::{
    fs::{File, OpenOptions},
    io::{ReadExt, WriteExt},
};
use bcs::{from_bytes, to_bytes};
use chrono::{DateTime, Utc};
use clap::Parser;
use shared::{
    http::protocol::{HttpRequest, HttpResponse},
    key_value::{KeyValueOperation, KeyValueOutput},
    platform::PlatformResponse,
    time::TimeResponse,
    Effect, Event, Request, ViewModel,
};
use std::{collections::VecDeque, time::SystemTime};

enum CoreMessage {
    Event(Event),
    Response(Vec<u8>, Outcome),
}

#[derive(Parser, Clone)]
enum Command {
    Clear,
    Get,
    Fetch,
}

pub enum Outcome {
    Platform(PlatformResponse),
    Time(TimeResponse),
    Http(HttpResponse),
    KeyValue(KeyValueOutput),
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

    let mut queue: VecDeque<CoreMessage> = VecDeque::new();

    queue.push_back(CoreMessage::Event(Event::Restore));
    queue.push_back(CoreMessage::Event(Event::GetPlatform));

    while !queue.is_empty() {
        let msg = queue.pop_front();

        let reqs = match msg {
            Some(CoreMessage::Event(m)) => shared::process_event(&to_bytes(&m)?),
            Some(CoreMessage::Response(uuid, output)) => shared::handle_response(
                &uuid,
                &match output {
                    Outcome::Platform(x) => to_bytes(&x)?,
                    Outcome::Time(x) => to_bytes(&x)?,
                    Outcome::Http(x) => to_bytes(&x)?,
                    Outcome::KeyValue(x) => to_bytes(&x)?,
                },
            ),
            _ => vec![],
        };
        let reqs: Vec<Request<Effect>> = from_bytes(&reqs)?;

        for req in reqs {
            let Request { uuid, effect } = req;
            match effect {
                Effect::Render(_) => (),
                Effect::Time(_) => {
                    let now: DateTime<Utc> = SystemTime::now().into();
                    let iso_time = now.to_rfc3339();

                    queue.push_back(CoreMessage::Response(
                        uuid,
                        Outcome::Time(TimeResponse(iso_time)),
                    ));
                }
                Effect::Http(HttpRequest { url, .. }) => match surf::get(&url).recv_bytes().await {
                    Ok(bytes) => {
                        queue.push_back(CoreMessage::Response(
                            uuid,
                            Outcome::Http(HttpResponse {
                                status: 200,
                                body: bytes,
                            }),
                        ));
                    }
                    Err(e) => bail!("Could not HTTP GET from {}: {}", &url, e),
                },
                Effect::Platform(_) => queue.push_back(CoreMessage::Response(
                    uuid,
                    Outcome::Platform(PlatformResponse("cli".to_string())),
                )),
                Effect::KeyValue(request) => match request {
                    KeyValueOperation::Read(key) => {
                        let bytes = read_state(&key).await.ok();

                        let initial_msg = match &args.cmd {
                            Command::Clear => CoreMessage::Event(Event::Clear),
                            Command::Get => CoreMessage::Event(Event::Get),
                            Command::Fetch => CoreMessage::Event(Event::Fetch),
                        };

                        queue.push_back(CoreMessage::Response(
                            uuid,
                            Outcome::KeyValue(KeyValueOutput::Read(bytes)),
                        ));
                        queue.push_back(initial_msg);
                    }
                    KeyValueOperation::Write(key, value) => {
                        let success = write_state(&key, &value).await.is_ok();

                        queue.push_back(CoreMessage::Response(
                            uuid,
                            Outcome::KeyValue(KeyValueOutput::Write(success)),
                        ));
                    }
                },
            }
        }
    }

    let view = from_bytes::<ViewModel>(&shared::view())?;
    println!("platform: {}", view.platform);
    println!("{}", view.fact);

    Ok(())
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
