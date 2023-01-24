use async_std::io::ReadExt;
use bcs::{from_bytes, to_bytes};
use clap::Parser;
use crossbeam_channel::Sender;
use eyre::{bail, eyre, Result};
use shared::{
    http::protocol::{HttpRequest, HttpResponse},
    sse::{SseRequest, SseResponse},
    Effect, Event, Request, ViewModel,
};
use std::{str::FromStr, time::Duration};
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
    Events,
}

impl From<Command> for CoreMessage {
    fn from(cmd: Command) -> Self {
        match cmd {
            Command::Get => CoreMessage::Message(Event::Get),
            Command::Inc => CoreMessage::Message(Event::Increment),
            Command::Dec => CoreMessage::Message(Event::Decrement),
            Command::Events => CoreMessage::Message(Event::GetServerEvents),
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

#[async_std::main]
async fn main() -> Result<()> {
    println!("^c to exit");
    let (tx, rx) = crossbeam_channel::unbounded::<CoreMessage>();

    let cmd = Args::parse().cmd;
    tx.send(cmd.into()).unwrap();

    let handle = async_task_group::group(|group| async move {
        while let Ok(msg) = rx.recv() {
            let reqs = match msg {
                CoreMessage::Message(m) => shared::message(&to_bytes(&m).unwrap()),
                CoreMessage::Response(uuid, output) => shared::response(
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
                        println!("{view:?}");
                    }
                    Effect::Http(HttpRequest { method, url }) => {
                        let method = Method::from_str(&method).expect("unknown http method");
                        let url = Url::parse(&url).unwrap();
                        let response = http(method, &url).await.unwrap();

                        tx.send(CoreMessage::Response(uuid, Outcome::Http(response)))
                            .unwrap();
                    }
                    Effect::ServerSentEvents(SseRequest { url }) => {
                        group.spawn({
                            let url = Url::parse(&url).unwrap();
                            let tx = tx.clone();
                            async move { get_sse(&uuid, &url, &tx).await }
                        });
                    }
                }
            }
        }
        Ok(group)
    });

    handle.await.unwrap();

    Ok(())
}

async fn http(method: Method, url: &Url) -> Result<HttpResponse> {
    let client: Client = Config::new()
        .set_timeout(Some(Duration::from_secs(5)))
        .try_into()?;

    let mut response = client
        .request(method, url)
        .await
        .map_err(|e| eyre!("{method} {url}: error {e}"))?;

    let status = response.status().into();

    if let 200..=299 = status {
        response.body_bytes().await.map_or_else(
            |e| bail!("{method} {url}: error {e}"),
            |body| Ok(HttpResponse { status, body }),
        )
    } else {
        bail!("{method} {url}: status {status}");
    }
}

async fn get_sse(uuid: &[u8], url: &Url, tx: &Sender<CoreMessage>) -> Result<()> {
    let mut response = surf::get(url)
        .await
        .map_err(|e| eyre!("get {url}: error {e}"))?;

    let status = response.status().into();

    let body = if let 200..=299 = status {
        response.take_body()
    } else {
        bail!("get {url}: status {status}");
    };

    let mut body = body.into_reader();
    let mut buf = [0; 1024];
    loop {
        match body.read(&mut buf).await {
            Ok(n) if n == 0 => {
                tx.send(CoreMessage::Response(
                    uuid.to_vec(),
                    Outcome::Sse(SseResponse::Done),
                ))
                .unwrap();
            }
            Ok(n) => {
                tx.send(CoreMessage::Response(
                    uuid.to_vec(),
                    Outcome::Sse(SseResponse::Chunk(buf[0..n].to_vec())),
                ))
                .unwrap();
            }
            Err(e) => {
                eprintln!("failed to read from http response; err = {:?}", e);
                break;
            }
        };
    }

    Ok(())
}
