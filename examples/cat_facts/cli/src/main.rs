use anyhow::{bail, Result};
use async_std::{
    fs::{File, OpenOptions},
    io::{ReadExt, WriteExt},
};
use chrono::{DateTime, Utc};
use clap::Parser;
use shared::*;
use std::{collections::VecDeque, time::SystemTime};

enum CoreMessage {
    Message(Event),
    Response(Response),
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

    let mut queue: VecDeque<CoreMessage> = VecDeque::new();

    queue.push_back(CoreMessage::Message(Event::Restore));

    while !queue.is_empty() {
        let msg = queue.pop_front();

        let reqs = match msg {
            Some(CoreMessage::Message(m)) => shared::message(&bcs::to_bytes(&m)?),
            Some(CoreMessage::Response(r)) => shared::response(&bcs::to_bytes(&r)?),
            _ => vec![],
        };
        let reqs: Vec<Request> = bcs::from_bytes(&reqs)?;

        for req in reqs {
            let Request { uuid, body } = req;
            match body {
                RequestBody::Render => (),
                RequestBody::Time => {
                    let now: DateTime<Utc> = SystemTime::now().into();
                    let iso_time = now.to_rfc3339();

                    queue.push_back(CoreMessage::Response(Response {
                        body: ResponseBody::Time(iso_time),
                        uuid,
                    }));
                }
                RequestBody::Http(url) => match surf::get(&url).recv_bytes().await {
                    Ok(bytes) => {
                        queue.push_back(CoreMessage::Response(Response {
                            body: ResponseBody::Http(bytes),
                            uuid,
                        }));
                    }
                    Err(e) => bail!("Could not HTTP GET from {}: {}", &url, e),
                },
                RequestBody::Platform => {}
                RequestBody::KVRead(key) => {
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
                RequestBody::KVWrite(key, val) => {
                    let success = write_state(&key, &val).await.is_ok();

                    queue.push_back(CoreMessage::Response(Response {
                        body: ResponseBody::KVWrite(success),
                        uuid,
                    }));
                }
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
