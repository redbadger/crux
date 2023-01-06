use anyhow::Result;
use clap::Parser;
use shared::{
    http::{HttpError, HttpRequest, HttpResponse},
    Effect, Event, Request, ViewModel,
};
use std::{collections::VecDeque, str::FromStr, time::Duration};
use surf::{http::Method, Client, Config, Url};

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
    Http(Result<HttpResponse, HttpError>),
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
                Effect::Render(_) => (),
                Effect::Http(HttpRequest { method, url }) => {
                    let method = Method::from_str(&method).expect("unknown http method");
                    let url = Url::parse(&url)?;

                    let response = http(method, &url).await;
                    queue.push_back(CoreMessage::Response(uuid, Outcome::Http(response)));
                }
            }
        }
    }

    let view = bcs::from_bytes::<ViewModel>(&shared::view())?;
    println!("{}", view.text);

    Ok(())
}

async fn http(method: Method, url: &Url) -> Result<HttpResponse, HttpError> {
    let error = |error| HttpError {
        method: method.to_string(),
        url: url.to_string(),
        error,
    };

    let client: Client = Config::new()
        .set_timeout(Some(Duration::from_secs(5)))
        .try_into()
        .map_err(|e| error(format!("{e}")))?;

    let mut response = client
        .request(method, url)
        .await
        .map_err(|e| error(format!("{e}")))?;

    let status = response.status().into();

    match status {
        200..=299 => response.body_bytes().await.map_or_else(
            |e| Err(error(format!("{e}"))),
            |body| {
                Ok(HttpResponse {
                    status,
                    body: Some(body),
                })
            },
        ),
        status => Err(error(format!("status: {status}"))),
    }
}
