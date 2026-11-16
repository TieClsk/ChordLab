//! #43: the delay send is tempo-synced, so echoes land exactly on the beat grid.
use engine_core::*;

/// One closed hi-hat at `beat`, fully sent to the delay; returns the left channel.
fn render_hit(tempo_bpm: f64, delay_beats: f64, beat: f64, sr: f32) -> Vec<f32> {
    let p = Project {
        tempo_bpm,
        master_gain: 1.0,
        delay_beats,
        tracks: vec![Track {
            name: "hat".into(),
            instrument: Instrument::Drums,
            gain: 1.0,
            pan: 0.0,
            mute: false,
            delay_send: 1.0,
            clips: vec![Clip {
                start_beat: 0.0,
                notes: vec![Note {
                    start: beat,
                    length: 0.1,
                    pitch: 42,
                    velocity: 127,
                }],
            }],
        }],
    };
    let (l, _) = render_offline(&p, vec![], sr, 2.0, 128);
    l
}

/// First non-silent sample after the dry hit has died out (= first echo).
fn first_echo(l: &[f32], dry_start: usize) -> usize {
    let dry_end = dry_start
        + l[dry_start..]
            .iter()
            .position(|x| *x == 0.0)
            .expect("dry hit ends");
    dry_end
        + l[dry_end..]
            .iter()
            .position(|x| *x != 0.0)
            .expect("an echo")
}

fn frames(beats: f64, bpm: f64, sr: f32) -> usize {
    (beats * 60.0 / bpm * sr as f64).round() as usize
}

#[test]
fn echo_lands_on_the_eighth_note_by_default() {
    let sr = 48_000.0;
    let l = render_hit(100.0, 0.5, 1.0, sr);
    let hit = frames(1.0, 100.0, sr);
    assert_eq!(first_echo(&l, hit), hit + frames(0.5, 100.0, sr)); // 0.3 s later
}

#[test]
fn echo_follows_the_tempo() {
    let sr = 48_000.0;
    for bpm in [72.0, 120.0, 140.0] {
        let l = render_hit(bpm, 0.5, 0.0, sr);
        assert_eq!(first_echo(&l, 0), frames(0.5, bpm, sr), "{bpm} BPM");
    }
}

#[test]
fn delay_length_is_configurable_in_beats() {
    let sr = 44_100.0;
    let l = render_hit(120.0, 0.75, 0.0, sr); // dotted eighth
    assert_eq!(first_echo(&l, 0), frames(0.75, 120.0, sr));
}

#[test]
fn delay_beats_defaults_to_an_eighth_note_for_older_projects() {
    let json = r#"{"tempo_bpm":100,"tracks":[]}"#;
    let p: Project = serde_json::from_str(json).unwrap();
    assert_eq!(p.delay_beats, 0.5);
}

#[test]
fn very_slow_tempos_are_clamped_not_reallocated_on_the_audio_thread() {
    // 4 beats at 20 BPM = 12 s: longer than the preallocated buffer; must not panic
    let l = render_hit(20.0, 4.0, 0.0, 48_000.0);
    assert!(l.iter().all(|x| x.is_finite()));
}
