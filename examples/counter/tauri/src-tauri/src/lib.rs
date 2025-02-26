mod http;
mod sse;

use anyhow::anyhow;
use futures::TryStreamExt;
use lazy_static::lazy_static;
use shared::{App, Core, Effect, Event};
use std::sync::Arc;
use tauri::Emitter;

lazy_static! {
    static ref CORE: Arc<Core<App>> = Arc::new(Core::new());
}

fn handle_event(
    event: Event,
    core: &Arc<Core<App>>,
    tauri_app: tauri::AppHandle,
) -> anyhow::Result<()> {
    for effect in core.process_event(event) {
        process_effect(effect, core, tauri_app.clone())?
    }

    Ok(())
}

fn process_effect(
    effect: Effect,
    core: &Arc<Core<App>>,
    tauri_app: tauri::AppHandle,
) -> anyhow::Result<()> {
    match effect {
        Effect::Render(_) => {
            let view = core.view();
            tauri_app.emit("render", view).map_err(|e| anyhow!(e))
        }
        Effect::Http(mut request) => {
            tauri::async_runtime::spawn({
                let core = core.clone();

                async move {
                    let response = http::request(&request.operation).await;
                    for effect in core
                        .resolve(&mut request, response.into())
                        .map_err(|e| anyhow!(e))?
                    {
                        process_effect(effect, &core, tauri_app.clone())?;
                    }

                    anyhow::Ok(())
                }
            });

            Ok(())
        }
        Effect::ServerSentEvents(mut request) => {
            tauri::async_runtime::spawn({
                let core = core.clone();

                async move {
                    let mut stream = sse::request(&request.operation).await?;

                    while let Ok(Some(response)) = stream.try_next().await {
                        for effect in core
                            .resolve(&mut request, response)
                            .map_err(|e| anyhow!(e))?
                        {
                            process_effect(effect, &core, tauri_app.clone())?;
                        }
                    }

                    anyhow::Ok(())
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
