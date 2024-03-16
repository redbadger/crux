mod http;
mod sse;

use anyhow::anyhow;
use futures::TryStreamExt;
use lazy_static::lazy_static;
use std::sync::Arc;
use tauri::Manager;

use shared::{App, Capabilities, Core, Effect, Event};

lazy_static! {
    static ref CORE: Arc<Core<Effect, App>> = Arc::new(Core::new::<Capabilities>());
}

fn handle_event(
    event: Event,
    core: &Arc<Core<Effect, App>>,
    tauri_app: tauri::AppHandle,
) -> anyhow::Result<()> {
    for effect in core.process_event(event) {
        process_effect(effect, core, tauri_app.clone())?
    }

    Ok(())
}

fn process_effect(
    effect: Effect,
    core: &Arc<Core<Effect, App>>,
    tauri_app: tauri::AppHandle,
) -> anyhow::Result<()> {
    match effect {
        Effect::Render(_) => {
            let view = core.view();
            tauri_app.emit_all("render", view).map_err(|e| anyhow!(e))
        }
        Effect::Http(mut request) => {
            tauri::async_runtime::spawn({
                let core = core.clone();

                async move {
                    let response = http::request(&request.operation).await;
                    for effect in core.resolve(&mut request, response.into()) {
                        let _ = process_effect(effect, &core, tauri_app.clone());
                    }
                }
            });

            Ok(())
        }
        Effect::ServerSentEvents(mut request) => {
            tauri::async_runtime::spawn({
                let core = core.clone();

                async move {
                    let mut stream = sse::request(&request.operation)
                        .await
                        .expect("error processing SSE effect");

                    while let Ok(Some(response)) = stream.try_next().await {
                        for effect in core.resolve(&mut request, response) {
                            let _ = process_effect(effect, &core, tauri_app.clone());
                        }
                    }
                }
            });

            Ok(())
        }
    }
}

#[tauri::command]
async fn increment(app_handle: tauri::AppHandle) {
    let _ = handle_event(Event::Increment, &CORE, app_handle);
}

#[tauri::command]
async fn decrement(app_handle: tauri::AppHandle) {
    let _ = handle_event(Event::Decrement, &CORE, app_handle);
}

#[tauri::command]
async fn watch(app_handle: tauri::AppHandle) {
    let _ = handle_event(Event::StartWatch, &CORE, app_handle);
}

/// The main entry point for Tauri
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![increment, decrement, watch])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
