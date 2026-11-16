//! Project (song) data model. Serialized as JSON between UI and engine.
//! Times are in beats so that tempo changes do not move notes.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../packages/engine-web/src/types/")
)]
pub struct Project {
    pub tempo_bpm: f64,
    #[serde(default = "default_gain")]
    pub master_gain: f32,
    /// Delay-send time in beats (tempo-synced). 0.5 = eighth note.
    #[serde(default = "default_delay_beats")]
    pub delay_beats: f64,
    pub tracks: Vec<Track>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../packages/engine-web/src/types/")
)]
pub struct Track {
    pub name: String,
    pub instrument: Instrument,
    #[serde(default = "default_gain")]
    pub gain: f32,
    /// -1.0 (left) ..= 1.0 (right)
    #[serde(default)]
    pub pan: f32,
    #[serde(default)]
    pub mute: bool,
    #[serde(default)]
    pub delay_send: f32,
    pub clips: Vec<Clip>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../packages/engine-web/src/types/")
)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Instrument {
    /// Subtractive synth: PolyBLEP saw/square -> state-variable low-pass -> ADSR.
    Synth(SynthParams),
    /// Multi-sample sampler. Each zone maps a sample id to a root note.
    Sampler {
        zones: Vec<SampleZone>,
        release_s: f32,
    },
    /// Synthesized drum kit (GM-ish note map: 36 kick, 38 snare, 42 closed hat).
    Drums,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../packages/engine-web/src/types/")
)]
pub struct SynthParams {
    #[serde(default)]
    pub wave: Wave,
    pub cutoff_hz: f32,
    pub resonance: f32,
    pub attack_s: f32,
    pub decay_s: f32,
    pub sustain: f32,
    pub release_s: f32,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Default)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../packages/engine-web/src/types/")
)]
#[serde(rename_all = "snake_case")]
pub enum Wave {
    #[default]
    Saw,
    Square,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../packages/engine-web/src/types/")
)]
pub struct SampleZone {
    pub sample_id: u32,
    pub root_note: u8,
    pub lo_note: u8,
    pub hi_note: u8,
    /// Velocity layer range (inclusive). Defaults cover the full range.
    #[serde(default)]
    pub lo_vel: u8,
    #[serde(default = "max_vel")]
    pub hi_vel: u8,
    /// Linear gain applied to this zone (layer balancing).
    #[serde(default = "unity")]
    pub gain: f32,
}

impl SampleZone {
    /// One sample stretched over the whole keyboard and velocity range.
    pub fn full(sample_id: u32, root_note: u8) -> Self {
        SampleZone {
            sample_id,
            root_note,
            lo_note: 0,
            hi_note: 127,
            lo_vel: 0,
            hi_vel: 127,
            gain: 1.0,
        }
    }
}

fn max_vel() -> u8 {
    127
}
fn unity() -> f32 {
    1.0
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../packages/engine-web/src/types/")
)]
pub struct Clip {
    pub start_beat: f64,
    pub notes: Vec<Note>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../packages/engine-web/src/types/")
)]
pub struct Note {
    /// relative to clip start, in beats
    pub start: f64,
    pub length: f64,
    pub pitch: u8,
    #[serde(default = "default_vel")]
    pub velocity: u8,
}

fn default_gain() -> f32 {
    0.8
}
fn default_delay_beats() -> f64 {
    0.5
}
fn default_vel() -> u8 {
    100
}

impl Project {
    pub fn beats_to_frames(&self, beats: f64, sample_rate: f64) -> u64 {
        (beats * 60.0 / self.tempo_bpm * sample_rate).round() as u64
    }

    /// Length of the project in beats (end of last note).
    pub fn length_beats(&self) -> f64 {
        self.tracks
            .iter()
            .flat_map(|t| {
                t.clips.iter().flat_map(move |c| {
                    c.notes
                        .iter()
                        .map(move |n| c.start_beat + n.start + n.length)
                })
            })
            .fold(0.0, f64::max)
    }
}
