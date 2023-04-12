use futures::TryStreamExt;
use lazy_static::lazy_static;
use shared::{
    sse::{SseRequest, SseResponse},
    App, Capabilities, Core, Effect, Event,
};
use std::sync::Arc;
use tauri::Manager;

mod http;
use http::http;

mod error;
use error::Error;

mod sse;
use sse::sse;

lazy_static! {
    static ref CORE: Arc<Core<Effect, App>> = Arc::new(Core::new::<Capabilities>());
}

fn handle_event(
    event: Event,
    core: &Arc<Core<Effect, App>>,
    tauri_app: tauri::AppHandle,
) -> Result<(), Error> {
    for effect in core.process_event(event) {
        process_effect(effect, core, tauri_app.clone())?
    }

    Ok(())
}

fn process_effect(
    effect: Effect,
    core: &Arc<Core<Effect, App>>,
    tauri_app: tauri::AppHandle,
) -> Result<(), Error> {
    match effect {
        Effect::Render(_) => {
            let view = core.view();
            tauri_app.emit_all("render", view)?
        }
        Effect::Http(mut request) => {
            tauri::async_runtime::spawn({
                let core = core.clone();

                async move {
                    let response = http(&request.operation)
                        .await
                        .expect("error processing Http effect");

                    for effect in core.resolve(&mut request, response) {
                        let _ = process_effect(effect, &core, tauri_app.clone());
                    }
                }
            });

            Ok::<(), Error>(())?
        }
        Effect::ServerSentEvents(mut request) => {
            let SseRequest { ref url } = request.operation;

            tauri::async_runtime::spawn({
                let core = core.clone();
                let url = url.clone();

                async move {
                    let mut stream = sse(url).await.expect("error processing SSE effect");

                    while let Ok(Some(item)) = stream.try_next().await {
                        let response = SseResponse::Chunk(item);
                        for effect in core.resolve(&mut request, response) {
                            let _ = process_effect(effect, &core, tauri_app.clone());
                        }
                    }
                }
            });

            Ok::<(), Error>(())?
        }
    };

    Ok(())
}

#[tauri::command]
async fn increment(app_handle: tauri::AppHandle) {
    let _ = handle_event(Event::Increment, &CORE, app_handle);
}

#[tauri::command]
async fn decrement(app_handle: tauri::AppHandle) {
    let _ = handle_event(Event::Decrement, &CORE, app_handle);
}

/// The main entry point for Tauri
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            handle_event(Event::StartWatch, &CORE, app.handle())
                .expect("error sending StartWatch Event");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![increment, decrement])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
