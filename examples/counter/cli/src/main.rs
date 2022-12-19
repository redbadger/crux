use anyhow::{bail, Result};
use clap::Parser;
use shared::{
    http::{HttpRequest, HttpResponse},
    Effect, Event, Request, ViewModel,
};
use std::{collections::VecDeque, str::FromStr, time::Duration};
use surf::{http, Client, Config, Url};

enum CoreMessage {
    Message(Event),
    Response(Vec<u8>, Outcome),
}

#[derive(Parser, Clone)]
enum Command {
    Get,
    Inc,
    Dec,
}

impl From<Command> for CoreMessage {
    fn from(cmd: Command) -> Self {
        match cmd {
            Command::Get => CoreMessage::Message(Event::Get),
            Command::Inc => CoreMessage::Message(Event::Increment),
            Command::Dec => CoreMessage::Message(Event::Decrement),
        }
    }
}

pub enum Outcome {
    Http(HttpResponse),
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    cmd: Command,
}

#[async_std::main]
async fn main() -> Result<()> {
    let mut queue: VecDeque<CoreMessage> = VecDeque::new();

    let cmd = Args::parse().cmd;
    queue.push_back(cmd.into());

    while !queue.is_empty() {
        let msg = queue.pop_front();

        let reqs = match msg {
            Some(CoreMessage::Message(m)) => shared::message(&bcs::to_bytes(&m)?),
            Some(CoreMessage::Response(uuid, output)) => shared::response(
                &uuid,
                &match output {
                    Outcome::Http(x) => bcs::to_bytes(&x)?,
                },
            ),
            _ => vec![],
        };
        let reqs: Vec<Request<Effect>> = bcs::from_bytes(&reqs)?;

        for Request { uuid, effect } in reqs {
            match effect {
                Effect::Render => (),
                Effect::Http(HttpRequest { url, method }) => {
                    let url = Url::parse(&url)?;
                    let method = http::Method::from_str(&method).expect("unknown http method");

                    let client: Client = Config::new()
                        .set_timeout(Some(Duration::from_secs(5)))
                        .try_into()?;

                    match client.request(method, &url).recv_bytes().await {
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
                    }
                }
            }
        }
    }

    let view = bcs::from_bytes::<ViewModel>(&shared::view())?;
    println!("{}", view.text);

    Ok(())
}
