//! Spike: load the public-domain "Splendid Grand Piano" SFZ set (AAC/m4a), map its
//! velocity layers to sampler zones, and render a progression.

use engine_core::*;
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

/// Minimal SFZ region reader: `<region> lokey=.. hikey=.. pitch_keycenter=.. sample=..`
/// `sample=` may contain spaces, so by SFZ convention it runs to the end of the line.
fn regions(text: &str) -> Vec<HashMap<String, String>> {
    text.split("<region>")
        .skip(1)
        .map(|r| {
            let line = r.lines().next().unwrap_or("");
            let mut m: HashMap<String, String> = HashMap::new();
            let mut rest = line;
            if let Some(i) = line.find("sample=") {
                let after = &line[i + 7..];
                // the file name ends at its extension; opcodes may follow it
                let end = after.find(".$EXT").map(|j| j + 5).unwrap_or(after.len());
                m.insert("sample".into(), after[..end].trim().to_string());
                let tail = &after[end..];
                for (k, v) in tail.split_whitespace().filter_map(|kv| kv.split_once('=')) {
                    m.insert(k.into(), v.into());
                }
                rest = &line[..i];
            }
            for (k, v) in rest.split_whitespace().filter_map(|kv| kv.split_once('=')) {
                m.insert(k.into(), v.into());
            }
            m
        })
        .collect()
}

pub struct PianoLoad {
    pub zones: Vec<SampleZone>,
    pub samples: Vec<(u32, Vec<f32>, f32)>,
    pub decode_ms: f64,
    pub audio_seconds: f64,
    pub bytes_f32: usize,
}

pub fn load(dir: &Path) -> PianoLoad {
    // (layer file, velocity range) as defined by the set's main .sfz groups
    let layers = [
        ("PP", 1u8, 67u8),
        ("MP", 68, 84),
        ("MF", 85, 100),
        ("FF", 101, 127),
    ];
    let mut ids: HashMap<String, u32> = HashMap::new();
    let mut zones = Vec::new();
    for (file, lo_vel, hi_vel) in layers {
        let text = std::fs::read_to_string(dir.join("data").join(format!("{file}.txt"))).unwrap();
        for r in regions(&text) {
            let name = r["sample"].replace("$EXT", "m4a");
            let next = ids.len() as u32;
            let id = *ids.entry(name).or_insert(next);
            zones.push(SampleZone {
                sample_id: id,
                root_note: r["pitch_keycenter"].parse().unwrap(),
                lo_note: r["lokey"].parse().unwrap(),
                hi_note: r["hikey"].parse().unwrap(),
                lo_vel,
                hi_vel,
                gain: 1.0,
            });
        }
    }
    let t = Instant::now();
    let mut samples = Vec::new();
    let mut secs = 0.0;
    let mut bytes = 0;
    let mut names: Vec<_> = ids.iter().collect();
    names.sort_by_key(|(_, id)| **id);
    for (name, id) in names {
        let path = dir.join("samples").join(name);
        let data = std::fs::read(&path).unwrap_or_else(|_| panic!("missing {path:?}"));
        let d = audio_io::decode(data, Some("m4a")).unwrap();
        secs += d.duration_s();
        let mono = d.to_mono();
        bytes += mono.len() * 4;
        samples.push((*id, mono, d.sample_rate as f32));
    }
    PianoLoad {
        zones,
        samples,
        decode_ms: t.elapsed().as_secs_f64() * 1e3,
        audio_seconds: secs,
        bytes_f32: bytes,
    }
}

/// I–V–vi–IV (C) as piano block chords with varying velocity, plus a bass note.
pub fn progression(zones: Vec<SampleZone>, bars: u32) -> Project {
    let chords: [&[u8]; 4] = [
        &[48, 60, 64, 67],
        &[43, 59, 62, 67],
        &[45, 60, 64, 69],
        &[41, 60, 65, 69],
    ];
    let mut notes = Vec::new();
    for b in 0..bars {
        let c = chords[(b % 4) as usize];
        for (beat, vel) in [(0.0, 96u8), (1.5, 70), (2.0, 84), (3.0, 60)] {
            for &p in c {
                notes.push(Note {
                    start: b as f64 * 4.0 + beat,
                    length: if beat == 0.0 { 1.4 } else { 0.9 },
                    pitch: p,
                    velocity: vel,
                });
            }
        }
    }
    Project {
        tempo_bpm: 96.0,
        master_gain: 0.9,
        delay_beats: 0.5,
        tracks: vec![Track {
            name: "Grand".into(),
            instrument: Instrument::Sampler {
                zones,
                release_s: 0.35,
            },
            gain: 0.7,
            pan: 0.0,
            mute: false,
            delay_send: 0.0,
            clips: vec![Clip {
                start_beat: 0.0,
                notes,
            }],
        }],
    }
}
