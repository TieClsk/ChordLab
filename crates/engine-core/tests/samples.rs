//! #8 INF-4: samples are stored as i16 (half the memory of f32) and may be stereo.
use engine_core::*;

const SR: f32 = 48_000.0;

fn sampler_project(pan: f32) -> Project {
    Project {
        tempo_bpm: 120.0,
        master_gain: 1.0,
        delay_beats: 0.5,
        tracks: vec![Track {
            name: "piano".into(),
            instrument: Instrument::Sampler { zones: vec![SampleZone::full(0, 60)], release_s: 0.05 },
            gain: 1.0,
            pan,
            mute: false,
            delay_send: 0.0,
            clips: vec![Clip { start_beat: 0.0, notes: vec![Note { start: 0.0, length: 0.5, pitch: 60, velocity: 127 }] }],
        }],
    }
}

/// 0.5 s ramp so values are easy to predict.
fn ramp(n: usize) -> Vec<f32> {
    (0..n).map(|i| (i as f32 / n as f32) * 0.5).collect()
}

#[test]
fn samples_use_two_bytes_per_value() {
    let mut e = Engine::new(SR);
    e.add_sample(0, vec![0.25; 48_000], SR); // 1 s mono
    assert_eq!(e.sample_memory_bytes(), 96_000);
    e.add_sample_i16(1, vec![0; 2 * 48_000], 2, SR); // 1 s stereo
    assert_eq!(e.sample_memory_bytes(), 96_000 + 192_000);
}

#[test]
fn replacing_a_sample_does_not_leak_its_memory_in_the_count() {
    let mut e = Engine::new(SR);
    e.add_sample(0, vec![0.0; 1000], SR);
    e.add_sample(0, vec![0.0; 10], SR);
    assert_eq!(e.sample_memory_bytes(), 20);
}

#[test]
fn f32_input_is_converted_to_i16_within_one_step() {
    let data = ramp(24_000);
    let mut e = Engine::new(SR);
    e.add_sample(0, data.clone(), SR);
    e.load_project(&sampler_project(0.0));
    e.apply(Command::Play);
    let (mut l, mut r) = (vec![0.0; 2000], vec![0.0; 2000]);
    e.process(&mut l, &mut r);
    // played at the root pitch, unity gains, centre pan (0.75), full velocity, attack 2 ms
    let pan = 0.75f32;
    for i in [200usize, 1000, 1999] {
        let expected = data[i] * pan;
        assert!((l[i] - expected).abs() < 2.0 / 32768.0 + 1e-6, "frame {i}: {} vs {}", l[i], expected);
    }
}

#[test]
fn stereo_sample_keeps_its_channels_apart() {
    // left = signal, right = silence
    let n = 24_000;
    let mut inter = vec![0i16; 2 * n];
    for i in 0..n {
        inter[2 * i] = 8_000;
    }
    let mut e = Engine::new(SR);
    e.add_sample_i16(0, inter, 2, SR);
    e.load_project(&sampler_project(0.0));
    e.apply(Command::Play);
    let (mut l, mut r) = (vec![0.0; 4800], vec![0.0; 4800]);
    e.process(&mut l, &mut r);
    assert!(l.iter().any(|x| x.abs() > 0.1), "left channel plays");
    assert!(r.iter().all(|x| *x == 0.0), "right channel stays silent");
}

#[test]
fn mono_sample_sounds_the_same_on_both_sides_when_centred() {
    let mut e = Engine::new(SR);
    e.add_sample(0, ramp(24_000), SR);
    e.load_project(&sampler_project(0.0));
    e.apply(Command::Play);
    let (mut l, mut r) = (vec![0.0; 4800], vec![0.0; 4800]);
    e.process(&mut l, &mut r);
    assert_eq!(l, r);
}

#[test]
fn stereo_sample_is_pitch_shifted_like_mono() {
    // an octave up plays twice as fast: a 2400-frame stereo sample ends after ~1200 frames
    let n = 2400;
    let inter: Vec<i16> = (0..2 * n).map(|_| 10_000).collect();
    let mut p = sampler_project(0.0);
    p.tracks[0].clips[0].notes[0].pitch = 72;
    let mut e = Engine::new(SR);
    e.add_sample_i16(0, inter, 2, SR);
    e.load_project(&p);
    e.apply(Command::Play);
    let (mut l, mut r) = (vec![0.0; 4800], vec![0.0; 4800]);
    e.process(&mut l, &mut r);
    let last = l.iter().rposition(|x| *x != 0.0).unwrap();
    assert!((1190..=1210).contains(&last), "ended at frame {last}");
}

#[test]
fn rejects_channel_counts_other_than_one_or_two() {
    let mut e = Engine::new(SR);
    assert!(!e.add_sample_i16(0, vec![0; 30], 3, SR));
    assert!(!e.add_sample_i16(0, vec![0; 31], 2, SR), "odd length cannot be interleaved stereo");
    assert!(e.add_sample_i16(0, vec![0; 30], 2, SR));
}
