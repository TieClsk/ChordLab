//! #9: loading a sample pack (manifest + audio files) into engine-ready samples.
use sample_packs::{load_pack, validate, Manifest, PackError};
use std::path::Path;

const SR: u32 = 44_100;

/// Minimal 16-bit PCM WAV writer for fixtures.
fn write_wav(path: &Path, channels: u16, frames: &[[i16; 2]]) {
    let mut b = Vec::new();
    let data_len = (frames.len() * channels as usize * 2) as u32;
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data_len).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&channels.to_le_bytes());
    b.extend_from_slice(&SR.to_le_bytes());
    b.extend_from_slice(&(SR * channels as u32 * 2).to_le_bytes());
    b.extend_from_slice(&(channels * 2).to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&data_len.to_le_bytes());
    for f in frames {
        for v in f.iter().take(channels as usize) {
            b.extend_from_slice(&v.to_le_bytes());
        }
    }
    std::fs::write(path, b).unwrap();
}

/// Two-sample pack: a 10 s stereo sample (L=+, R=-) for keys 21..64, a 1 s mono one for 65..108.
fn fixture(dir: &Path) {
    std::fs::create_dir_all(dir.join("samples")).unwrap();
    let long: Vec<[i16; 2]> = (0..10 * SR).map(|_| [10_000, -10_000]).collect();
    write_wav(&dir.join("samples/low.wav"), 2, &long);
    let short: Vec<[i16; 2]> = (0..SR).map(|_| [5_000, 0]).collect();
    write_wav(&dir.join("samples/high.wav"), 1, &short);
    let manifest = serde_json::json!({
        "name": "fixture", "version": 1, "license": "test", "credit": "test",
        "source": {"repo": "x", "commit": "y"},
        "max_seconds": 6.0, "fade_seconds": 0.5, "release_s": 0.3,
        "samples": [
            {"file": "samples/low.wav", "layer": "A", "root_note": 48},
            {"file": "samples/high.wav", "layer": "A", "root_note": 72}
        ],
        "zones": [
            {"sample_id": 0, "root_note": 48, "lo_note": 21, "hi_note": 64, "lo_vel": 1, "hi_vel": 127, "gain": 1.0},
            {"sample_id": 1, "root_note": 72, "lo_note": 65, "hi_note": 108, "lo_vel": 1, "hi_vel": 127, "gain": 1.0}
        ]
    });
    std::fs::write(
        dir.join("manifest.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();
}

fn tmp(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("chordlab-pack-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

#[test]
fn long_samples_are_trimmed_to_max_seconds_with_a_fade_out() {
    let d = tmp("trim");
    fixture(&d);
    let p = load_pack(&d).unwrap();
    let low = &p.samples[0];
    assert_eq!(low.channels, 2);
    assert_eq!(low.data.len() / 2, 6 * SR as usize, "trimmed to 6 s");
    let frame = |i: usize| (low.data[2 * i], low.data[2 * i + 1]);
    assert_eq!(frame(0), (10_000, -10_000), "start untouched");
    assert_eq!(
        frame(5 * SR as usize),
        (10_000, -10_000),
        "before the fade untouched"
    );
    let (l, r) = frame(6 * SR as usize - 1);
    assert!(
        l.abs() < 50 && r.abs() < 50,
        "faded to silence at the end: {l} {r}"
    );
    let mid = frame((5.75 * SR as f64) as usize).0;
    assert!(
        (4_000..6_000).contains(&mid),
        "half-way through the fade: {mid}"
    );
}

#[test]
fn short_samples_keep_their_length_and_mono_stays_mono() {
    let d = tmp("short");
    fixture(&d);
    let p = load_pack(&d).unwrap();
    let high = &p.samples[1];
    assert_eq!(high.channels, 1);
    assert_eq!(high.data.len(), SR as usize);
    assert_eq!(high.sample_rate, SR as f32);
}

#[test]
fn reports_memory_and_exposes_zones_for_the_engine() {
    let d = tmp("mem");
    fixture(&d);
    let p = load_pack(&d).unwrap();
    assert_eq!(p.memory_bytes(), 6 * SR as usize * 2 * 2 + SR as usize * 2);
    assert_eq!(p.zones.len(), 2);
    assert_eq!(p.release_s, 0.3);
    let mut e = engine_core::Engine::new(48_000.0);
    p.add_to(&mut e, 0);
    assert_eq!(e.sample_memory_bytes(), p.memory_bytes());
}

#[test]
fn sample_ids_can_be_offset_when_several_packs_are_loaded() {
    let d = tmp("offset");
    fixture(&d);
    let p = load_pack(&d).unwrap();
    let zones = p.zones_with_offset(100);
    assert_eq!(zones[0].sample_id, 100);
    assert_eq!(zones[1].sample_id, 101);
}

#[test]
fn validate_rejects_gaps_overlaps_and_bad_references() {
    let d = tmp("validate");
    fixture(&d);
    let ok: Manifest =
        serde_json::from_slice(&std::fs::read(d.join("manifest.json")).unwrap()).unwrap();
    assert!(validate(&ok).is_ok());

    let mut gap = ok.clone();
    gap.zones[1].lo_note = 66; // key 65 uncovered
    assert!(matches!(
        validate(&gap),
        Err(PackError::Coverage { key: 65, .. })
    ));

    let mut overlap = ok.clone();
    overlap.zones[1].lo_note = 64; // key 64 covered twice
    assert!(matches!(
        validate(&overlap),
        Err(PackError::Coverage { key: 64, .. })
    ));

    let mut bad_ref = ok.clone();
    bad_ref.zones[0].sample_id = 9;
    assert!(matches!(validate(&bad_ref), Err(PackError::Manifest(_))));
}

#[test]
fn missing_file_is_a_clear_error() {
    let d = tmp("missing");
    fixture(&d);
    std::fs::remove_file(d.join("samples/high.wav")).unwrap();
    match load_pack(&d) {
        Err(PackError::Io { file, .. }) => assert!(file.ends_with("high.wav")),
        other => panic!("expected Io error, got {:?}", other.map(|_| ())),
    }
}

/// Real Lite piano pack (built by scripts/packs/build_piano_lite.py). Run with
/// `PIANO_PACK_DIR=<dir> cargo test -p sample-packs --test load -- --ignored --nocapture`.
#[test]
#[ignore]
fn real_lite_piano_pack_fits_the_memory_budget() {
    let dir = std::env::var("PIANO_PACK_DIR").expect("PIANO_PACK_DIR");
    let t = std::time::Instant::now();
    let p = load_pack(Path::new(&dir)).unwrap();
    let ms = t.elapsed().as_secs_f64() * 1e3;
    let mb = p.memory_bytes() as f64 / 1e6;
    println!(
        "PACK_REPORT samples={} memory_mb={mb:.1} load_ms={ms:.0}",
        p.samples.len()
    );
    assert!(mb <= 60.0, "ADR-0005 budget: {mb:.1} MB > 60 MB");
    assert!(
        p.samples.iter().all(|s| s.channels == 2),
        "piano samples keep their stereo image"
    );
}
