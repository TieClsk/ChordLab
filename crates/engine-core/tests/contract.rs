//! Cross-language contract: the TypeScript bridge must encode engine commands exactly
//! like `Command::encode`. This test owns the fixture consumed by
//! `packages/engine-web/test/commands.test.ts`.
//!
//! The fixture is checked in. If the encoding changes, this test fails until the
//! fixture is regenerated with `UPDATE_FIXTURES=1 cargo test -p engine-core --test contract`,
//! which then makes the TS test fail until the TS encoder is updated too.
use engine_core::Command;
use serde_json::{json, Value};

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../packages/engine-web/test/fixtures/commands.json");

/// (TS-side description, Rust command)
fn cases() -> Vec<(Value, Command)> {
    vec![
        (json!({"type": "play"}), Command::Play),
        (json!({"type": "stop"}), Command::Stop),
        (json!({"type": "seek", "frame": 0}), Command::Seek { frame: 0 }),
        (json!({"type": "seek", "frame": 5_000_000_123u64}), Command::Seek { frame: 5_000_000_123 }),
        (json!({"type": "noteOn", "track": 3, "pitch": 61, "velocity": 99}), Command::NoteOn { track: 3, pitch: 61, velocity: 99 }),
        (json!({"type": "noteOff", "track": 3, "pitch": 61}), Command::NoteOff { track: 3, pitch: 61 }),
        (json!({"type": "setTrackGain", "track": 1, "gain": 0.33}), Command::SetTrackGain { track: 1, gain: 0.33 }),
        (json!({"type": "setTrackPan", "track": 1, "pan": -0.5}), Command::SetTrackPan { track: 1, pan: -0.5 }),
        (json!({"type": "setMute", "track": 2, "mute": true}), Command::SetMute { track: 2, mute: true }),
        (json!({"type": "setMute", "track": 2, "mute": false}), Command::SetMute { track: 2, mute: false }),
        (json!({"type": "setLoop", "start": 10, "end": 20}), Command::SetLoop { start: 10, end: 20 }),
    ]
}

fn generate() -> String {
    let arr: Vec<Value> = cases().into_iter().map(|(desc, cmd)| json!({"command": desc, "words": cmd.encode()})).collect();
    serde_json::to_string_pretty(&arr).unwrap() + "\n"
}

#[test]
fn command_fixture_is_up_to_date() {
    let fresh = generate();
    if std::env::var("UPDATE_FIXTURES").is_ok() {
        std::fs::create_dir_all(std::path::Path::new(FIXTURE).parent().unwrap()).unwrap();
        std::fs::write(FIXTURE, &fresh).unwrap();
    }
    let on_disk = std::fs::read_to_string(FIXTURE).unwrap_or_default();
    assert_eq!(on_disk, fresh, "command fixture is stale: run UPDATE_FIXTURES=1 cargo test -p engine-core --test contract");
}
