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

#[async_std::main]
async fn main() -> Result<()> {
    let (tx, rx) = crossbeam_channel::unbounded::<CoreMessage>();

    let cmd = Args::parse().cmd;

    let watching = matches!(cmd, Command::Watch);
    if watching {
        println!("^C to exit");
    }

    tx.send(cmd.into()).unwrap();

    let handle = async_task_group::group(|group| async move {
        'outer: while let Ok(msg) = rx.recv() {
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
                        if !view.text.contains("pending") {
                            println!("{}", view.text);
                            if !watching {
                                break 'outer;
                            }
                        }
                    }
                    Effect::Http(HttpRequest { method, url }) => {
                        group.spawn({
                            let method = Method::from_str(&method).expect("unknown http method");
                            let url = Url::parse(&url).unwrap();

                            http(uuid, method, url, tx.clone())
                        });
                    }
                    Effect::ServerSentEvents(SseRequest { url }) => {
                        group.spawn({
                            let url = Url::parse(&url).unwrap();

                            sse(uuid, url, tx.clone())
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

async fn http(uuid: Vec<u8>, method: Method, url: Url, tx: Sender<CoreMessage>) -> Result<()> {
    let client: Client = Config::new()
        .set_timeout(Some(Duration::from_secs(5)))
        .try_into()?;

    let mut response = client
        .request(method, &url)
        .await
        .map_err(|e| eyre!("{method} {url}: error {e}"))?;

    let status = response.status().into();

    match status {
        200..=299 => match response.body_bytes().await {
            Ok(body) => {
                tx.send(CoreMessage::Response(
                    uuid.to_vec(),
                    Outcome::Http(HttpResponse { status, body }),
                ))?;
            }
            Err(e) => bail!("{method} {url}: error {e}"),
        },
        _ => bail!("{method} {url}: status {status}"),
    };
    Ok(())
}

async fn sse(uuid: Vec<u8>, url: Url, tx: Sender<CoreMessage>) -> Result<()> {
    let mut response = surf::get(&url)
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
        let response = match body.read(&mut buf).await {
            Ok(n) if n == 0 => SseResponse::Done,
            Ok(n) => SseResponse::Chunk(buf[0..n].to_vec()),
            Err(e) => bail!("failed to read from http response; err = {:?}", e),
        };
        tx.send(CoreMessage::Response(uuid.to_vec(), Outcome::Sse(response)))?;
    }
}
