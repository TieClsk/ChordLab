//! ChordLab audio engine core.
//!
//! Target-agnostic: compiled natively for the desktop app (driven by cpal) and to
//! `wasm32-unknown-unknown` for the browser (driven by an AudioWorklet).
//! See `docs/architecture/overview.md` and ADR-0001.

pub mod dsp;
pub mod engine;
pub mod model;

pub use engine::{render_offline, Command, Engine, Event, MAX_VOICES};
pub use model::*;

/// A small demo project used by tests, benches and spikes: I–V–vi–IV in C with
/// pad synth, bass, drums (and an optional sampler piano track).
pub fn demo_project(with_sampler: bool, bars: u32) -> Project {
    let prog: [[u8; 3]; 4] = [[60, 64, 67], [59, 62, 67], [57, 60, 64], [57, 60, 65]];
    let roots = [36u8, 43, 45, 41];
    let mut chords = Vec::new();
    let mut bass = Vec::new();
    let mut drums = Vec::new();
    for b in 0..bars {
        let k = (b % 4) as usize;
        let s = b as f64 * 4.0;
        for &p in &prog[k] {
            chords.push(Note {
                start: s,
                length: 3.9,
                pitch: p,
                velocity: 90,
            });
        }
        for e in 0..8 {
            bass.push(Note {
                start: s + e as f64 * 0.5,
                length: 0.45,
                pitch: roots[k],
                velocity: if e % 2 == 0 { 110 } else { 80 },
            });
        }
        for beat in 0..4 {
            let t = s + beat as f64;
            drums.push(Note {
                start: t,
                length: 0.1,
                pitch: if beat % 2 == 0 { 36 } else { 38 },
                velocity: 120,
            });
            drums.push(Note {
                start: t,
                length: 0.1,
                pitch: 42,
                velocity: 70,
            });
            drums.push(Note {
                start: t + 0.5,
                length: 0.1,
                pitch: 42,
                velocity: 60,
            });
        }
    }
    let synth = |cut: f32, a: f32, r: f32, wave: Wave| SynthParams {
        wave,
        cutoff_hz: cut,
        resonance: 0.3,
        attack_s: a,
        decay_s: 0.3,
        sustain: 0.7,
        release_s: r,
    };
    let mut tracks = vec![
        Track {
            name: "Pad".into(),
            instrument: Instrument::Synth(synth(2200.0, 0.08, 0.6, Wave::Saw)),
            gain: 0.5,
            pan: -0.2,
            mute: false,
            delay_send: 0.25,
            clips: vec![Clip {
                start_beat: 0.0,
                notes: chords.clone(),
            }],
        },
        Track {
            name: "Bass".into(),
            instrument: Instrument::Synth(synth(700.0, 0.005, 0.08, Wave::Square)),
            gain: 0.6,
            pan: 0.0,
            mute: false,
            delay_send: 0.0,
            clips: vec![Clip {
                start_beat: 0.0,
                notes: bass,
            }],
        },
        Track {
            name: "Drums".into(),
            instrument: Instrument::Drums,
            gain: 0.7,
            pan: 0.0,
            mute: false,
            delay_send: 0.05,
            clips: vec![Clip {
                start_beat: 0.0,
                notes: drums,
            }],
        },
    ];
    if with_sampler {
        let up: Vec<Note> = chords
            .iter()
            .map(|n| Note {
                pitch: n.pitch + 12,
                ..n.clone()
            })
            .collect();
        tracks.push(Track {
            name: "Piano".into(),
            instrument: Instrument::Sampler {
                zones: vec![SampleZone::full(0, 60)],
                release_s: 0.3,
            },
            gain: 0.6,
            pan: 0.3,
            mute: false,
            delay_send: 0.1,
            clips: vec![Clip {
                start_beat: 0.0,
                notes: up,
            }],
        });
    }
    Project {
        tempo_bpm: 120.0,
        master_gain: 0.8,
        delay_beats: 0.5,
        tracks,
    }
}

/// Deterministic test sample (decaying harmonic tone at middle C) so tests do not
/// need audio files.
pub fn test_tone_sample(sample_rate: f32) -> Vec<f32> {
    let n = (sample_rate * 2.0) as usize;
    let f = 261.625_58 / sample_rate;
    let mut o1 = dsp::Osc::default();
    let mut o2 = dsp::Osc::default();
    let mut lvl = 1.0f32;
    let d = 1.0 - 1.0 / (0.6 * sample_rate);
    (0..n)
        .map(|_| {
            lvl *= d;
            (o1.sine(f) + 0.4 * o2.sine(2.0 * f)) * lvl * 0.6
        })
        .collect()
}
