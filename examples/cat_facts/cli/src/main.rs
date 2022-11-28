use anyhow::{bail, Result};
use async_std::{
    fs::{File, OpenOptions},
    io::{ReadExt, WriteExt},
};
use chrono::{DateTime, Utc};
use clap::Parser;
use std::{collections::VecDeque, time::SystemTime};

use shared::{http, time, Effect, Event, Request, Response};

enum CoreMessage<T> {
    Message(Event),
    Response(Vec<u8>, Response<T>),
}

#[derive(Parser, Debug, Clone)]
enum Command {
    Clear,
    Get,
    Fetch,
}

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    cmd: Command,
}

#[async_std::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let mut queue: VecDeque<CoreMessage<_>> = VecDeque::new();

    queue.push_back(CoreMessage::Message(Event::Restore));

    while !queue.is_empty() {
        let msg = queue.pop_front();

        let reqs = match msg {
            Some(CoreMessage::Message(m)) => shared::message(&bcs::to_bytes(&m)?),
            Some(CoreMessage::Response(uuid, output)) => {
                shared::response(&uuid, &bcs::to_bytes(&output)?)
            }
            _ => vec![],
        };
        let reqs: Vec<Request<Effect>> = bcs::from_bytes(&reqs)?;

        for req in reqs {
            let Request { uuid, effect } = req;
            match effect {
                Effect::Render => (),
                Effect::Time => {
                    let now: DateTime<Utc> = SystemTime::now().into();
                    let iso_time = now.to_rfc3339();

                    queue.push_back(CoreMessage::Response(
                        uuid,
                        Response {
                            body: time::Response(iso_time),
                        },
                    ));
                }
                Effect::Http(http::Request { url, .. }) => match surf::get(&url).recv_bytes().await
                {
                    Ok(bytes) => {
                        queue.push_back(CoreMessage::Response(
                            uuid,
                            Response {
                                body: http::Response {
                                    status: 200,
                                    body: bytes,
                                },
                            },
                        ));
                    }
                    Err(e) => bail!("Could not HTTP GET from {}: {}", &url, e),
                },
                Effect::Platform => {}
                Effect::KeyValue(request) => match request {
                    crux_core::key_value::Request::Read(key) => {
                        let bytes = read_state(&key).await.ok();

                        let initial_msg = match &args.cmd {
                            Command::Clear => CoreMessage::Message(Event::Clear),
                            Command::Get => CoreMessage::Message(Event::Get),
                            Command::Fetch => CoreMessage::Message(Event::Fetch),
                        };

                        queue.push_back(CoreMessage::Response(Response {
                            body: ResponseBody::KVRead(bytes),
                            uuid,
                        }));
                        queue.push_back(initial_msg);
                    }
                    crux_core::key_value::Request::Write(key, value) => {
                        let success = write_state(&key, &val).await.is_ok();

                        queue.push_back(CoreMessage::Response(Response {
                            body: ResponseBody::KVWrite(success),
                            uuid,
                        }));
                    }
                },
            }
        }
    }

    let view = shared::view();
    println!("{}", bcs::from_bytes::<ViewModel>(&view)?.fact);

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
