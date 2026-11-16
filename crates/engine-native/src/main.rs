//! chordlab-engine CLI — spike / export tool.
//!
//!   chordlab-engine render <demo|project.json> <out.wav> [bars]
//!   chordlab-engine raw <out.f32>            # demo, 48 kHz, block 128, interleaved f32 LE
//!   chordlab-engine bench                    # CPU cost vs polyphony
//!   chordlab-engine rt-sim                   # audio thread + UI thread via lock-free queue

mod piano;
mod wav;

use engine_core::*;
use std::time::{Duration, Instant};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("piano") => {
            let pl = piano::load(std::path::Path::new(&args[2]));
            println!(
                "piano: {} zones, {} samples decoded in {:.0} ms ({:.0} s of audio, {:.1} MB as mono f32)",
                pl.zones.len(),
                pl.samples.len(),
                pl.decode_ms,
                pl.audio_seconds,
                pl.bytes_f32 as f64 / 1e6
            );
            let p = piano::progression(pl.zones, 8);
            std::fs::write(
                args[3].replace(".wav", ".json"),
                serde_json::to_string(&p).unwrap(),
            )
            .unwrap();
            let t = Instant::now();
            let (l, r) = render_offline(&p, pl.samples, 48_000.0, 1.5, 512);
            println!(
                "rendered {:.1} s in {:.0} ms",
                l.len() as f64 / 48_000.0,
                t.elapsed().as_secs_f64() * 1e3
            );
            wav::write_wav16(&args[3], 48_000, &l, &r).unwrap();
        }
        Some("render") => render(
            &args[2],
            &args[3],
            args.get(4).and_then(|s| s.parse().ok()).unwrap_or(16),
        ),
        Some("raw") => raw(&args[2]),
        Some("bench") => bench(),
        Some("rt-sim") => rt_sim(),
        _ => eprintln!("usage: chordlab-engine render|raw|bench|rt-sim ..."),
    }
}

fn render(src: &str, out: &str, bars: u32) {
    let sr = 48_000.0;
    let p = if src == "demo" {
        demo_project(true, bars)
    } else {
        serde_json::from_str(&std::fs::read_to_string(src).unwrap()).unwrap()
    };
    let t = Instant::now();
    let (l, r) = render_offline(&p, vec![(0, test_tone_sample(sr), sr)], sr, 1.0, 512);
    let dt = t.elapsed();
    wav::write_wav16(out, sr as u32, &l, &r).unwrap();
    let secs = l.len() as f64 / sr as f64;
    println!(
        "rendered {secs:.1} s of audio in {:.1} ms (x{:.0} realtime) -> {out}",
        dt.as_secs_f64() * 1e3,
        secs / dt.as_secs_f64()
    );
}

fn raw(out: &str) {
    let sr = 48_000.0;
    let (l, r) = render_offline(
        &demo_project(true, 8),
        vec![(0, test_tone_sample(sr), sr)],
        sr,
        1.0,
        128,
    );
    let mut bytes = Vec::with_capacity(l.len() * 8);
    for i in 0..l.len() {
        bytes.extend_from_slice(&l[i].to_le_bytes());
        bytes.extend_from_slice(&r[i].to_le_bytes());
    }
    std::fs::write(out, bytes).unwrap();
    println!("{} frames", l.len());
}

/// Project that keeps exactly `n` voices sounding for the whole render.
fn poly_project(n: usize, kind: &str) -> Project {
    let inst = match kind {
        "sampler" => Instrument::Sampler {
            zones: vec![SampleZone::full(0, 60)],
            release_s: 0.2,
        },
        _ => Instrument::Synth(SynthParams {
            wave: Wave::Saw,
            cutoff_hz: 3000.0,
            resonance: 0.3,
            attack_s: 0.01,
            decay_s: 0.2,
            sustain: 0.7,
            release_s: 0.2,
        }),
    };
    let notes = (0..n)
        .map(|i| Note {
            start: 0.0,
            length: 3.5,
            pitch: 36 + (i % 60) as u8,
            velocity: 90,
        })
        .collect();
    Project {
        tempo_bpm: 120.0,
        master_gain: 0.2,
        delay_beats: 0.5,
        tracks: vec![Track {
            name: "t".into(),
            instrument: inst,
            gain: 0.5,
            pan: 0.0,
            mute: false,
            delay_send: 0.1,
            clips: vec![Clip {
                start_beat: 0.0,
                notes,
            }],
        }],
    }
}

fn bench() {
    let sr = 48_000.0f32;
    let block = 128;
    let budget_us = block as f64 / sr as f64 * 1e6;
    println!("block {block} @ {sr} Hz -> budget {budget_us:.0} µs per block");
    for kind in ["synth", "sampler"] {
        for n in [8usize, 16, 32, 64] {
            let mut e = Engine::new(sr);
            e.add_sample(0, test_tone_sample(sr), sr);
            e.load_project(&poly_project(n, kind));
            e.apply(Command::Play);
            let mut l = vec![0.0; block];
            let mut r = vec![0.0; block];
            let blocks = sr as usize / block; // ~1 s (notes sustain the whole time)
            let mut worst = Duration::ZERO;
            let t = Instant::now();
            for _ in 0..blocks {
                let b = Instant::now();
                e.process(&mut l, &mut r);
                worst = worst.max(b.elapsed());
            }
            let avg = t.elapsed().as_secs_f64() * 1e6 / blocks as f64;
            println!(
                "{kind:8} voices={n:2} avg {avg:6.1} µs ({:5.1}% of budget) worst {:6.1} µs",
                100.0 * avg / budget_us,
                worst.as_secs_f64() * 1e6
            );
        }
    }
}

/// Simulates the desktop architecture without an audio device: a UI thread pushes
/// 16-byte commands into a wait-free SPSC ring (rtrb); the "audio" thread drains it
/// at the start of every block, renders, and reports how late it ever was.
fn rt_sim() {
    let sr = 48_000.0f32;
    let block = 128usize;
    let period = Duration::from_secs_f64(block as f64 / sr as f64);
    let (mut tx, mut rx) = rtrb::RingBuffer::<[u32; 4]>::new(1024);
    let audio = std::thread::spawn(move || {
        let mut e = Engine::new(sr);
        e.add_sample(0, test_tone_sample(sr), sr);
        e.load_project(&demo_project(true, 16));
        let (mut l, mut r) = (vec![0.0; block], vec![0.0; block]);
        let start = Instant::now();
        let mut worst_render = Duration::ZERO;
        let mut cmds = 0u64;
        let mut late_blocks = 0u64;
        for k in 0..(sr as usize * 4 / block) {
            let deadline = start + period * k as u32;
            if Instant::now() > deadline + period {
                late_blocks += 1;
            }
            while Instant::now() < deadline {
                std::hint::spin_loop();
            }
            // commands are applied at the next block boundary: latency <= 1 block
            while let Ok(w) = rx.pop() {
                if let Some(c) = Command::decode(w) {
                    e.apply(c);
                    cmds += 1;
                }
            }
            let t = Instant::now();
            e.process(&mut l, &mut r);
            worst_render = worst_render.max(t.elapsed());
        }
        (worst_render, cmds, late_blocks)
    });
    // UI thread: start playback, then fire 2000 note on/off pairs and gain changes
    tx.push(Command::Play.encode()).unwrap();
    let mut dropped = 0;
    for i in 0..2000u32 {
        for c in [
            Command::NoteOn {
                track: 0,
                pitch: 60 + (i % 24) as u8,
                velocity: 90,
            },
            Command::NoteOff {
                track: 0,
                pitch: 60 + (i % 24) as u8,
            },
            Command::SetTrackGain {
                track: 1,
                gain: 0.5 + (i % 10) as f32 * 0.01,
            },
        ] {
            if tx.push(c.encode()).is_err() {
                dropped += 1;
            }
        }
        std::thread::sleep(Duration::from_micros(1500));
    }
    let (worst, cmds, late) = audio.join().unwrap();
    println!(
        "rt-sim: {cmds} commands applied, {dropped} dropped (queue full), {late} late blocks, worst block render {:.1} µs of {:.0} µs budget; command latency <= 1 block ({:.2} ms)",
        worst.as_secs_f64() * 1e6,
        period.as_secs_f64() * 1e6,
        block as f64 / sr as f64 * 1e3
    );
}
