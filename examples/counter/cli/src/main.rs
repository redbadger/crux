use async_std::io::ReadExt;
use clap::Parser;
use crossbeam_channel::Sender;
use eyre::{bail, eyre, Result};
use futures::{stream, TryStreamExt};
use shared::{
    http::protocol::{HttpHeader, HttpRequest, HttpResponse},
    sse::{SseRequest, SseResponse},
    App, Capabilities, Core, Effect, Event,
};
use std::{
    str::FromStr,
    sync::{Arc, Weak},
    time::Duration,
};
use surf::{http::Method, Client, Config, Url};

#[derive(Debug)]
enum Task {
    Event(Event),
    Effect(Effect),
}

#[derive(Parser, Clone)]
enum Command {
    Get,
    Inc,
    Dec,
    Watch,
}

impl From<Command> for Task {
    fn from(cmd: Command) -> Self {
        match cmd {
            Command::Get => Task::Event(Event::Get),
            Command::Inc => Task::Event(Event::Increment),
            Command::Dec => Task::Event(Event::Decrement),
            Command::Watch => Task::Event(Event::StartWatch),
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
    let (tx, rx) = crossbeam_channel::unbounded::<Task>();

    let strong_tx = Arc::new(tx);
    let tx = Arc::downgrade(&strong_tx);

    let core: Arc<Core<Effect, App>> = Arc::new(Core::new::<Capabilities>());

    // Kick off with the given command
    main_loop(&core, Args::parse().cmd.into(), tx.clone())?;
    drop(strong_tx); // tx may still live in a side-effect futures

    // Continue until there's no more work to do
    while let Ok(msg) = rx.recv() {
        main_loop(&core, msg, tx.clone())?;
    }

    Ok(())
}

fn main_loop(
    core: &Arc<Core<Effect, App>>,
    task: Task,
    tx: Weak<Sender<Task>>,
) -> Result<(), eyre::ErrReport> {
    match task {
        Task::Event(event) => {
            for effect in core.process_event(event) {
                process_effect(effect, core, tx.clone())?
            }
        }
        Task::Effect(effect) => process_effect(effect, core, tx)?,
    }

    Ok(())
}

fn process_effect(
    effect: Effect,
    core: &Arc<Core<Effect, App>>,
    tx: Weak<Sender<Task>>,
) -> Result<(), eyre::ErrReport> {
    match effect {
        Effect::Render(_) => {
            let text = core.view().text;

            if !text.contains("pending") {
                println!("{text}");
            }
        }
        Effect::Http(mut request) => {
            let HttpRequest {
                ref method,
                ref url,
                ref headers,
                body: _,
            } = request.operation;

            async_std::task::spawn({
                let method = Method::from_str(method).expect("unknown http method");
                let url = Url::parse(url)?;
                let headers = headers.clone();

                let core = core.clone();
                let tx = tx.upgrade().expect("Should be able to upgrade Weak tx");

                async move {
                    let response = http(method, url, &headers).await.unwrap();
                    for effect in core.resolve(&mut request, response) {
                        tx.send(Task::Effect(effect)).unwrap();
                    }
                }
            });
        }
        Effect::ServerSentEvents(mut request) => {
            let SseRequest { ref url } = request.operation;

            async_std::task::spawn({
                let url = Url::parse(url)?;

                let core = core.clone();
                let tx = tx.upgrade().unwrap();

                async move {
                    let mut stream = sse(url).await.unwrap();

                    while let Ok(Some(item)) = stream.try_next().await {
                        let response = SseResponse::Chunk(item);
                        for effect in core.resolve(&mut request, response) {
                            tx.send(Task::Effect(effect)).unwrap();
                        }
                    }
                }
            });
        }
    };

    Ok(())
}

async fn http(method: Method, url: Url, headers: &[HttpHeader]) -> Result<HttpResponse> {
    let client: Client = Config::new()
        .set_timeout(Some(Duration::from_secs(5)))
        .try_into()?;

    let mut request = client.request(method, &url);

    for header in headers {
        request = request.header(header.name.as_str(), &header.value);
    }

    let mut response = request
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
