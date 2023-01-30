use async_std::io::ReadExt;
use crossbeam_channel::Sender;
use futures::{stream, TryStreamExt};

use bcs::{from_bytes, to_bytes};
use clap::Parser;
use eyre::{bail, eyre, Result};
use shared::{
    http::protocol::{HttpRequest, HttpResponse},
    sse::{SseRequest, SseResponse},
    Effect, Event, Request, ViewModel,
};
use std::{
    str::FromStr,
    sync::{Arc, Weak},
    time::Duration,
};
use surf::{http::Method, Client, Config, Url};

enum CoreMessage {
    Event(Event),
    Response(Vec<u8>, Outcome),
}

#[derive(Parser, Clone)]
enum Command {
    Get,
    Inc,
    Dec,
    Watch,
}

impl From<Command> for CoreMessage {
    fn from(cmd: Command) -> Self {
        match cmd {
            Command::Get => CoreMessage::Event(Event::Get),
            Command::Inc => CoreMessage::Event(Event::Increment),
            Command::Dec => CoreMessage::Event(Event::Decrement),
            Command::Watch => CoreMessage::Event(Event::StartWatch),
        }
    }
}

pub enum Outcome {
    Http(HttpResponse),
    Sse(SseResponse),
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    cmd: Command,
}

fn main() -> Result<()> {
    let (tx, rx) = crossbeam_channel::unbounded::<CoreMessage>();

    let strong_tx = Arc::new(tx);
    let tx = Arc::downgrade(&strong_tx);

    // Kick off with the given command

    main_loop(Args::parse().cmd.into(), tx.clone())?;
    drop(strong_tx); // tx may still live in a side-effect futures

    // Continue until there's no more work to do
    while let Ok(msg) = rx.recv() {
        main_loop(msg, tx.clone())?;
    }

    Ok(())
}

fn main_loop(msg: CoreMessage, tx: Weak<Sender<CoreMessage>>) -> Result<(), eyre::ErrReport> {
    let reqs = match msg {
        CoreMessage::Event(m) => shared::process_event(&to_bytes(&m).unwrap()),
        CoreMessage::Response(uuid, output) => shared::handle_response(
            &uuid,
            &match output {
                Outcome::Http(x) => to_bytes(&x).unwrap(),
                Outcome::Sse(x) => to_bytes(&x).unwrap(),
            },
        ),
    };
    let reqs: Vec<Request<Effect>> = from_bytes(&reqs).unwrap();

    for Request { uuid, effect } in reqs {
        match effect {
            Effect::Render(_) => {
                let view = from_bytes::<ViewModel>(&shared::view())?;
                let text = view.text;

                if !text.contains("pending") {
                    println!("{text}");
                }
            }
            Effect::Http(HttpRequest { method, url }) => {
                let method = Method::from_str(&method).expect("unknown http method");
                let url = Url::parse(&url)?;

                async_std::task::spawn({
                    let tx = tx.upgrade().unwrap();
                    async move {
                        let response = http(method, url).await.unwrap();
                        let outcome = Outcome::Http(response);
                        let message = CoreMessage::Response(uuid.clone(), outcome);

                        tx.send(message).unwrap();
                    }
                });
            }
            Effect::ServerSentEvents(SseRequest { url }) => {
                let url = Url::parse(&url)?;

                async_std::task::spawn({
                    let tx = tx.upgrade().unwrap();
                    async move {
                        let mut stream = sse(url).await.unwrap();

                        while let Ok(Some(item)) = stream.try_next().await {
                            let outcome = Outcome::Sse(SseResponse::Chunk(item));
                            let message = CoreMessage::Response(uuid.clone(), outcome);

                            tx.send(message).unwrap();
                        }
                    }
                });
            }
        }
    }

    Ok(())
}

async fn http(method: Method, url: Url) -> Result<HttpResponse> {
    let client: Client = Config::new()
        .set_timeout(Some(Duration::from_secs(5)))
        .try_into()?;

    let mut response = client
        .request(method, &url)
        .await
        .map_err(|e| eyre!("{method} {url}: error {e}"))?;

    let status = response.status().into();

    match response.body_bytes().await {
        Ok(body) => Ok(HttpResponse { status, body }),
        Err(e) => bail!("{method} {url}: error {e}"),
    }
}

async fn sse(url: Url) -> Result<impl futures::stream::TryStream<Ok = Vec<u8>>> {
    let mut response = surf::get(&url)
        .await
        .map_err(|e| eyre!("get {url}: error {e}"))?;

    let status = response.status().into();

    let body = if let 200..=299 = status {
        response.take_body()
    } else {
        bail!("get {url}: status {status}");
    };

    let body = body.into_reader();

    Ok(Box::pin(stream::try_unfold(body, |mut body| async {
        let mut buf = [0; 1024];

        match body.read(&mut buf).await {
            Ok(n) if n == 0 => Ok(None),
            Ok(n) => Ok(Some((buf[0..n].to_vec(), body))),
            Err(e) => bail!("failed to read from http response; err = {:?}", e),
        }
    })))
}
