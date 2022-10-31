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

    match &args.cmd {
        Command::Clear => queue.push_back(CoreMessage::Message(Msg::Clear)),
        Command::Get => queue.push_back(CoreMessage::Message(Msg::Get)),
        Command::Fetch => queue.push_back(CoreMessage::Message(Msg::Fetch)),
    };

    while !queue.is_empty() {
        let msg = queue.pop_front();

        let reqs = match msg {
            Some(CoreMessage::Message(m)) => core.message(m),
            Some(CoreMessage::Response(r)) => core.response(r),
            _ => vec![],
        };

        for req in reqs {
            match req {
                Request::Http { uuid, url } => {
                    let bytes: Vec<u8> = surf::get(url).recv_bytes().await.unwrap();

                    queue.push_back(CoreMessage::Response(Response::Http { uuid, bytes }));
                }
                Request::Time { uuid } => {
                    let now: DateTime<Utc> = SystemTime::now().into();
                    let iso_time = now.to_rfc3339();

                    queue.push_back(CoreMessage::Response(Response::Time { uuid, iso_time }));
                }
                Request::Render => (),
            }
        }
    }

    let view = core.view();
    println!("{}", view.fact);
}
