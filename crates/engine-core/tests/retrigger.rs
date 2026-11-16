//! #42: repeated NoteOn for a pitch that is already held must not stack voices,
//! otherwise one NoteOff leaves notes ringing forever.
use engine_core::*;

fn synth_project() -> Project {
    Project {
        tempo_bpm: 120.0,
        master_gain: 0.8,
        tracks: vec![Track {
            name: "s".into(),
            instrument: Instrument::Synth(SynthParams {
                wave: Wave::Saw,
                cutoff_hz: 2000.0,
                resonance: 0.2,
                attack_s: 0.005,
                decay_s: 0.1,
                sustain: 0.7,
                release_s: 0.05,
            }),
            gain: 0.5,
            pan: 0.0,
            mute: false,
            delay_send: 0.0,
            clips: vec![],
        }],
    }
}

fn run(e: &mut Engine, seconds: f32) {
    let (mut l, mut r) = (vec![0.0; 128], vec![0.0; 128]);
    for _ in 0..(seconds * e.sample_rate() / 128.0) as usize {
        e.process(&mut l, &mut r);
    }
}

#[test]
fn repeated_note_on_retriggers_instead_of_stacking() {
    let mut e = Engine::new(48_000.0);
    e.load_project(&synth_project());
    for _ in 0..5 {
        e.apply(Command::NoteOn { track: 0, pitch: 60, velocity: 100 });
        run(&mut e, 0.02);
    }
    assert_eq!(e.active_voices(), 1, "same pitch held five times should be one voice");
    e.apply(Command::NoteOff { track: 0, pitch: 60 });
    run(&mut e, 0.5);
    assert_eq!(e.active_voices(), 0, "one NoteOff must silence the note");
}

#[test]
fn chord_held_with_key_repeat_releases_cleanly() {
    // simulates OS key auto-repeat on a chord button: C E G sent 20 times, released once
    let mut e = Engine::new(48_000.0);
    e.load_project(&synth_project());
    for _ in 0..20 {
        for p in [60, 64, 67] {
            e.apply(Command::NoteOn { track: 0, pitch: p, velocity: 100 });
        }
        run(&mut e, 0.03);
    }
    assert_eq!(e.active_voices(), 3);
    for p in [60, 64, 67] {
        e.apply(Command::NoteOff { track: 0, pitch: p });
    }
    run(&mut e, 0.5);
    assert_eq!(e.active_voices(), 0);
}

#[test]
fn same_pitch_on_different_tracks_is_independent() {
    let mut p = synth_project();
    p.tracks.push(p.tracks[0].clone());
    let mut e = Engine::new(48_000.0);
    e.load_project(&p);
    e.apply(Command::NoteOn { track: 0, pitch: 60, velocity: 100 });
    e.apply(Command::NoteOn { track: 1, pitch: 60, velocity: 100 });
    run(&mut e, 0.02);
    assert_eq!(e.active_voices(), 2);
    e.apply(Command::NoteOff { track: 0, pitch: 60 });
    run(&mut e, 0.5);
    assert_eq!(e.active_voices(), 1);
}

#[test]
fn a_released_note_can_ring_out_while_the_same_pitch_is_played_again() {
    // legato playing: the release tail of the old note overlaps the new one
    let mut e = Engine::new(48_000.0);
    let mut p = synth_project();
    if let Instrument::Synth(s) = &mut p.tracks[0].instrument {
        s.release_s = 1.0;
    }
    e.load_project(&p);
    e.apply(Command::NoteOn { track: 0, pitch: 60, velocity: 100 });
    run(&mut e, 0.05);
    e.apply(Command::NoteOff { track: 0, pitch: 60 });
    run(&mut e, 0.05);
    e.apply(Command::NoteOn { track: 0, pitch: 60, velocity: 100 });
    run(&mut e, 0.02);
    assert_eq!(e.active_voices(), 2, "releasing tail + new note");
    e.apply(Command::NoteOff { track: 0, pitch: 60 });
    run(&mut e, 1.5);
    assert_eq!(e.active_voices(), 0);
}
