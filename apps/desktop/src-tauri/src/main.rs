// ChordLab desktop shell. The UI talks to the native engine through Tauri commands;
// the engine runs on the cpal audio thread (see engine-native::realtime).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use engine_core::{Engine, Project};
use engine_native::realtime::Host;
use std::sync::atomic::Ordering;
use std::sync::Mutex;

struct AppState {
    host: Mutex<Option<Host>>,
}

/// Hot path: one 16-byte engine command, same encoding as the web build.
#[tauri::command]
fn engine_cmd(state: tauri::State<AppState>, w: [u32; 4]) -> bool {
    state
        .host
        .lock()
        .unwrap()
        .as_mut()
        .map(|h| h.command_raw(w))
        .unwrap_or(false)
}

#[tauri::command]
fn engine_status(state: tauri::State<AppState>) -> serde_json::Value {
    let g = state.host.lock().unwrap();
    match g.as_ref() {
        Some(h) => {
            let s = &h.status;
            serde_json::json!({
                "device": h.device_name, "sampleRate": h.sample_rate,
                "position": s.position.load(Ordering::Relaxed), "voices": s.voices.load(Ordering::Relaxed),
                "callbacks": s.callbacks.load(Ordering::Relaxed), "maxCallbackUs": s.max_callback_ns.load(Ordering::Relaxed) as f64 / 1e3,
                "framesPerCallback": s.last_frames.load(Ordering::Relaxed)
            })
        }
        None => serde_json::json!({ "error": "no audio device" }),
    }
}

/// Replaces the whole project (ADR-0004). The new engine is built here, off the
/// audio thread, then swapped in; transport is stopped after a load.
#[tauri::command]
fn load_project(state: tauri::State<AppState>, json: String) -> bool {
    let Ok(project) = serde_json::from_str::<Project>(&json) else {
        return false;
    };
    let mut g = state.host.lock().unwrap();
    let Some(h) = g.as_mut() else { return false };
    let mut e = Engine::new(h.sample_rate as f32);
    e.load_project(&project);
    h.replace_engine(e)
}

/// The UI asks this on startup; CI sets CHORDLAB_SELFTEST to run the desktop smoke test.
#[tauri::command]
fn selftest_requested() -> bool {
    std::env::var("CHORDLAB_SELFTEST").is_ok()
}

/// Receives the self-test result from the UI, prints it for CI and quits in self-test mode.
#[tauri::command]
fn report(app: tauri::AppHandle, json: String) {
    println!("SELFTEST_REPORT {json}");
    if selftest_requested() {
        app.exit(0);
    }
}

fn main() {
    let host = match Host::start() {
        Ok(h) => Some(h),
        Err(e) => {
            eprintln!("audio unavailable: {e}");
            None
        }
    };
    tauri::Builder::default()
        .manage(AppState {
            host: Mutex::new(host),
        })
        .invoke_handler(tauri::generate_handler![
            engine_cmd,
            engine_status,
            load_project,
            selftest_requested,
            report
        ])
        .run(tauri::generate_context!())
        .expect("error while running ChordLab");
}
