// ChordLab desktop shell. The UI talks to the native engine through Tauri commands;
// the engine runs on the cpal audio thread (see engine-native::realtime).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use engine_core::{demo_project, test_tone_sample, Command, Engine};
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

/// Builds a loaded engine off the audio thread, then swaps it in.
#[tauri::command]
fn load_demo(state: tauri::State<AppState>, bars: u32) -> bool {
    let mut g = state.host.lock().unwrap();
    let Some(h) = g.as_mut() else { return false };
    let sr = h.sample_rate as f32;
    let mut e = Engine::new(sr);
    e.add_sample(0, test_tone_sample(sr), sr);
    e.load_project(&demo_project(true, bars));
    e.apply(Command::Play);
    h.replace_engine(e)
}

/// Spike helper: print the UI-side measurements and quit (used by the CI smoke run).
#[tauri::command]
fn report(app: tauri::AppHandle, json: String) {
    println!("SPIKE_REPORT {json}");
    if std::env::var("CHORDLAB_SPIKE_EXIT").is_ok() {
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
            load_demo,
            report
        ])
        .run(tauri::generate_context!())
        .expect("error while running ChordLab");
}
