use engine_core::*;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

// Counting allocator: proves `Engine::process` never allocates.
// Tracking is per thread, because the test harness runs other tests in parallel.
struct Counting;
thread_local! {
    static TRACK: Cell<bool> = const { Cell::new(false) };
    static ALLOCS: Cell<usize> = const { Cell::new(0) };
}
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        if TRACK.with(|t| t.get()) {
            ALLOCS.with(|a| a.set(a.get() + 1));
        }
        System.alloc(l)
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        System.dealloc(p, l)
    }
}
#[global_allocator]
static A: Counting = Counting;

#[test]
fn process_is_allocation_free() {
    let mut e = Engine::new(48_000.0);
    e.add_sample(0, test_tone_sample(48_000.0), 48_000.0);
    e.load_project(&demo_project(true, 8));
    e.apply(Command::Play);
    let mut l = vec![0.0; 128];
    let mut r = vec![0.0; 128];
    TRACK.with(|t| t.set(true));
    for _ in 0..(48_000 * 8 / 128) {
        e.process(&mut l, &mut r);
        e.apply(Command::NoteOn {
            track: 0,
            pitch: 72,
            velocity: 100,
        });
        e.apply(Command::NoteOff {
            track: 0,
            pitch: 72,
        });
    }
    TRACK.with(|t| t.set(false));
    assert_eq!(
        ALLOCS.with(|a| a.get()),
        0,
        "process() allocated on the audio path"
    );
}

#[test]
fn scheduling_is_sample_accurate() {
    // one drum hit at beat 1.0 @ 120 bpm, 48 kHz -> frame 24000 exactly
    let p = Project {
        tempo_bpm: 120.0,
        master_gain: 1.0,
        tracks: vec![Track {
            name: "d".into(),
            instrument: Instrument::Drums,
            gain: 1.0,
            pan: 0.0,
            mute: false,
            delay_send: 0.0,
            clips: vec![Clip {
                start_beat: 0.0,
                notes: vec![Note {
                    start: 1.0,
                    length: 0.1,
                    pitch: 42,
                    velocity: 127,
                }],
            }],
        }],
    };
    for block in [1usize, 37, 128, 1024] {
        let (l, _) = render_offline(&p, vec![], 48_000.0, 0.1, block);
        let first = l.iter().position(|x| *x != 0.0).unwrap();
        assert_eq!(first, 24_000, "block size {block}");
    }
}

#[test]
fn output_independent_of_block_size() {
    let p = demo_project(true, 4);
    let s = || vec![(0u32, test_tone_sample(44_100.0), 44_100.0f32)];
    let (a, _) = render_offline(&p, s(), 44_100.0, 0.5, 128);
    let (b, _) = render_offline(&p, s(), 44_100.0, 0.5, 333);
    assert_eq!(a, b);
}

#[test]
fn command_roundtrip() {
    let cmds = [
        Command::Play,
        Command::Stop,
        Command::Seek {
            frame: 5_000_000_123,
        },
        Command::NoteOn {
            track: 3,
            pitch: 61,
            velocity: 99,
        },
        Command::NoteOff {
            track: 3,
            pitch: 61,
        },
        Command::SetTrackGain {
            track: 1,
            gain: 0.33,
        },
        Command::SetTrackPan {
            track: 1,
            pan: -0.5,
        },
        Command::SetMute {
            track: 2,
            mute: true,
        },
        Command::SetLoop { start: 10, end: 20 },
    ];
    for c in cmds {
        assert_eq!(Command::decode(c.encode()), Some(c));
    }
}

#[test]
fn project_json_roundtrip() {
    let p = demo_project(true, 2);
    let j = serde_json::to_string(&p).unwrap();
    let q: Project = serde_json::from_str(&j).unwrap();
    assert_eq!(p, q);
}

#[test]
fn loop_wraps() {
    let mut e = Engine::new(48_000.0);
    e.load_project(&demo_project(false, 4));
    e.apply(Command::SetLoop {
        start: 0,
        end: 96_000,
    });
    e.apply(Command::Play);
    let mut l = vec![0.0; 1000];
    let mut r = vec![0.0; 1000];
    for _ in 0..200 {
        e.process(&mut l, &mut r);
        assert!(e.position() < 96_000);
    }
}
