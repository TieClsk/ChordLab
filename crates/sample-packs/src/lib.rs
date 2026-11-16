//! ChordLab sample packs (#9, ADR-0005, ADR-0008).
//!
//! A pack is a directory with `manifest.json` and compressed audio files. Loading
//! decodes every file (symphonia, pure Rust), trims it to `max_seconds` with a linear
//! fade-out, and stores it as i16 (mono or interleaved stereo) ready for
//! `Engine::add_sample_i16`. Runs off the audio thread; works natively and in wasm32.

use engine_core::{Engine, SampleZone};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Source {
    pub repo: String,
    pub commit: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SampleEntry {
    pub file: String,
    pub layer: String,
    pub root_note: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Manifest {
    pub name: String,
    pub version: u32,
    pub license: String,
    pub credit: String,
    pub source: Source,
    pub max_seconds: f32,
    pub fade_seconds: f32,
    pub release_s: f32,
    pub samples: Vec<SampleEntry>,
    pub zones: Vec<SampleZone>,
}

#[derive(Debug)]
pub enum PackError {
    Io {
        file: PathBuf,
        error: std::io::Error,
    },
    Decode {
        file: PathBuf,
        error: String,
    },
    Manifest(String),
    /// A (key, velocity) is covered by `count` zones instead of exactly one.
    Coverage {
        key: u8,
        velocity: u8,
        count: usize,
    },
}

impl std::fmt::Display for PackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PackError::Io { file, error } => write!(f, "{}: {error}", file.display()),
            PackError::Decode { file, error } => write!(f, "{}: {error}", file.display()),
            PackError::Manifest(m) => write!(f, "manifest: {m}"),
            PackError::Coverage {
                key,
                velocity,
                count,
            } => {
                write!(
                    f,
                    "key {key} velocity {velocity} is covered by {count} zones (expected 1)"
                )
            }
        }
    }
}

impl std::error::Error for PackError {}

/// One decoded, trimmed sample.
#[derive(Clone, Debug)]
pub struct PackSample {
    /// i16 PCM, interleaved when `channels` == 2.
    pub data: Vec<i16>,
    pub channels: u8,
    pub sample_rate: f32,
}

pub struct LoadedPack {
    pub manifest: Manifest,
    pub samples: Vec<PackSample>,
    pub zones: Vec<SampleZone>,
    pub release_s: f32,
}

impl LoadedPack {
    pub fn memory_bytes(&self) -> usize {
        self.samples.iter().map(|s| s.data.len() * 2).sum()
    }

    /// Zones with sample ids shifted by `offset` (several packs in one engine).
    pub fn zones_with_offset(&self, offset: u32) -> Vec<SampleZone> {
        self.zones
            .iter()
            .map(|z| SampleZone {
                sample_id: z.sample_id + offset,
                ..z.clone()
            })
            .collect()
    }

    /// Copies every sample into the engine at ids `offset..`.
    pub fn add_to(&self, engine: &mut Engine, offset: u32) {
        for (i, s) in self.samples.iter().enumerate() {
            engine.add_sample_i16(offset + i as u32, s.data.clone(), s.channels, s.sample_rate);
        }
    }
}

/// Every key 21..=108 and velocity 1..=127 must hit exactly one zone, and every
/// zone must reference an existing sample.
pub fn validate(m: &Manifest) -> Result<(), PackError> {
    for (i, z) in m.zones.iter().enumerate() {
        if z.sample_id as usize >= m.samples.len() {
            return Err(PackError::Manifest(format!(
                "zone {i} references missing sample {}",
                z.sample_id
            )));
        }
        if z.lo_note > z.hi_note || z.lo_vel > z.hi_vel {
            return Err(PackError::Manifest(format!("zone {i} has an empty range")));
        }
    }
    for key in 21u8..=108 {
        for velocity in 1u8..=127 {
            let count = m
                .zones
                .iter()
                .filter(|z| {
                    (z.lo_note..=z.hi_note).contains(&key)
                        && (z.lo_vel..=z.hi_vel).contains(&velocity)
                })
                .count();
            if count != 1 {
                return Err(PackError::Coverage {
                    key,
                    velocity,
                    count,
                });
            }
        }
    }
    Ok(())
}

/// Trims to `max_frames` and fades the last `fade_frames` linearly to silence.
fn trim_and_fade(channels: &mut [Vec<f32>], max_frames: usize, fade_frames: usize) {
    for ch in channels.iter_mut() {
        if ch.len() <= max_frames {
            continue;
        }
        ch.truncate(max_frames);
        let fade = fade_frames.min(max_frames);
        let start = max_frames - fade;
        for (i, x) in ch[start..].iter_mut().enumerate() {
            *x *= 1.0 - (i + 1) as f32 / fade as f32;
        }
    }
}

fn to_i16(x: f32) -> i16 {
    (x * 32768.0) as i16 // saturating
}

/// Loads a pack from a directory (desktop, tests, build tools).
pub fn load_pack(dir: &Path) -> Result<LoadedPack, PackError> {
    let manifest_path = dir.join("manifest.json");
    let bytes = std::fs::read(&manifest_path).map_err(|error| PackError::Io {
        file: manifest_path.clone(),
        error,
    })?;
    let manifest: Manifest =
        serde_json::from_slice(&bytes).map_err(|e| PackError::Manifest(e.to_string()))?;
    load_with(manifest, |file| {
        let path = dir.join(file);
        std::fs::read(&path).map_err(|error| PackError::Io { file: path, error })
    })
}

/// Loads a pack whose files are supplied by `read` (e.g. fetched bytes in the browser).
pub fn load_with(
    manifest: Manifest,
    mut read: impl FnMut(&str) -> Result<Vec<u8>, PackError>,
) -> Result<LoadedPack, PackError> {
    validate(&manifest)?;
    let mut samples = Vec::with_capacity(manifest.samples.len());
    for entry in &manifest.samples {
        let bytes = read(&entry.file)?;
        let ext = Path::new(&entry.file).extension().and_then(|e| e.to_str());
        let decoded = audio_io::decode(bytes, ext).map_err(|e| PackError::Decode {
            file: PathBuf::from(&entry.file),
            error: format!("{e:?}"),
        })?;
        let sr = decoded.sample_rate;
        let mut chans = decoded.channels;
        if chans.is_empty() || chans.len() > 2 {
            return Err(PackError::Decode {
                file: PathBuf::from(&entry.file),
                error: format!("{} channels", chans.len()),
            });
        }
        let max_frames = (manifest.max_seconds * sr as f32) as usize;
        let fade_frames = (manifest.fade_seconds * sr as f32) as usize;
        trim_and_fade(&mut chans, max_frames, fade_frames);
        let frames = chans[0].len();
        let data: Vec<i16> = if chans.len() == 2 {
            (0..frames)
                .flat_map(|i| [to_i16(chans[0][i]), to_i16(chans[1][i])])
                .collect()
        } else {
            chans[0].iter().map(|x| to_i16(*x)).collect()
        };
        samples.push(PackSample {
            data,
            channels: chans.len() as u8,
            sample_rate: sr as f32,
        });
    }
    Ok(LoadedPack {
        zones: manifest.zones.clone(),
        release_s: manifest.release_s,
        samples,
        manifest,
    })
}
