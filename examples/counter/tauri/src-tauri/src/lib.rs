use shared::{Core, Counter, Effect, Event};
use std::sync::{Arc, LazyLock};
use tauri::Emitter;

static CORE: LazyLock<Arc<Core<Counter>>> = LazyLock::new(|| Arc::new(Core::new()));

fn handle_event(event: Event, core: &Arc<Core<Counter>>, app: &tauri::AppHandle) {
    for effect in core.process_event(event) {
        process_effect(&effect, core, app);
    }
}

fn process_effect(effect: &Effect, core: &Arc<Core<Counter>>, app: &tauri::AppHandle) {
    match effect {
        Effect::Render(_) => {
            let view = core.view();
            let _ = app.emit("render", view);
        }
    }
}

#[tauri::command]
async fn increment(app_handle: tauri::AppHandle) {
    handle_event(Event::Increment, &CORE, &app_handle);
}

#[tauri::command]
async fn decrement(app_handle: tauri::AppHandle) {
    handle_event(Event::Decrement, &CORE, &app_handle);
}

#[tauri::command]
async fn reset(app_handle: tauri::AppHandle) {
    handle_event(Event::Reset, &CORE, &app_handle);
}

/// The main entry point for Tauri
/// # Panics
/// If the Tauri application fails to run.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![increment, decrement, reset])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
