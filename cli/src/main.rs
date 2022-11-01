use anyhow::Result;
use async_std::{
    fs::{File, OpenOptions},
    io::{ReadExt, WriteExt},
};
use chrono::{DateTime, Utc};
use clap::Parser;
use std::{collections::VecDeque, time::SystemTime};

use shared::*;

enum CoreMessage {
    Message(Msg),
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
async fn main() {
    let args = Args::parse();

    let core = Core::new();
    let mut queue: VecDeque<CoreMessage> = VecDeque::new();

    queue.push_back(CoreMessage::Message(Msg::Restore));

    while !queue.is_empty() {
        let msg = queue.pop_front();

        let reqs = match msg {
            Some(CoreMessage::Message(m)) => core.message(m),
            Some(CoreMessage::Response(r)) => core.response(r),
            _ => vec![],
        };

        for req in reqs {
            match req {
                Request::Http {
                    data: StringEnvelope { uuid, body: url },
                } => {
                    let bytes: Vec<u8> = surf::get(url).recv_bytes().await.unwrap();

                    queue.push_back(CoreMessage::Response(Response::Http {
                        data: BytesEnvelope { uuid, body: bytes },
                    }));
                }
                Request::Platform { .. } => {}
                Request::Time {
                    data: OptionalBoolEnvelope { uuid, body: _ },
                } => {
                    let now: DateTime<Utc> = SystemTime::now().into();
                    let iso_time = now.to_rfc3339();

                    queue.push_back(CoreMessage::Response(Response::Time {
                        data: StringEnvelope {
                            uuid,
                            body: iso_time,
                        },
                    }));
                }
                Request::KVRead {
                    data: StringEnvelope { uuid, body: key },
                } => {
                    let bytes = read_state(&key).await.ok();

                    let initial_msg = match &args.cmd {
                        Command::Clear => CoreMessage::Message(Msg::Clear),
                        Command::Get => CoreMessage::Message(Msg::Get),
                        Command::Fetch => CoreMessage::Message(Msg::Fetch),
                    };

                    queue.push_back(CoreMessage::Response(Response::KVRead {
                        data: OptionalBytesEnvelope { uuid, body: bytes },
                    }));
                    queue.push_back(initial_msg);
                }
                Request::KVWrite {
                    data:
                        KeyValueEnvelope {
                            uuid,
                            body: KeyValue { key, value: bytes },
                        },
                } => {
                    let success = write_state(&key, &bytes).await.is_ok();

                    queue.push_back(CoreMessage::Response(Response::KVWrite {
                        data: BoolEnvelope {
                            uuid,
                            body: success,
                        },
                    }));
                }
                Request::Render => (),
            }
        }
    }

    let view = core.view();
    println!("{}", view.fact);
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
