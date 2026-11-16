//! Spike: how expensive is "rebuild the engine's event list from the whole project"
//! on every edit? If it is cheap enough, v1 can skip incremental engine updates.
use engine_core::*;
use std::time::Instant;

fn big_project(tracks: usize, notes_per_track: usize) -> Project {
    let tracks = (0..tracks)
        .map(|t| Track {
            name: format!("t{t}"),
            instrument: Instrument::Drums,
            gain: 0.5,
            pan: 0.0,
            mute: false,
            delay_send: 0.0,
            clips: vec![Clip {
                start_beat: 0.0,
                notes: (0..notes_per_track)
                    .map(|i| Note {
                        start: i as f64 * 0.25,
                        length: 0.2,
                        pitch: (36 + (i * 7 + t) % 60) as u8,
                        velocity: 90,
                    })
                    .collect(),
            }],
        })
        .collect();
    Project {
        tempo_bpm: 120.0,
        master_gain: 0.8,
        tracks,
    }
}

#[test]
fn reload_cost() {
    for (t, n) in [(8, 500), (32, 2000), (64, 4000)] {
        let p = big_project(t, n);
        let json = serde_json::to_string(&p).unwrap();
        let t0 = Instant::now();
        let q: Project = serde_json::from_str(&json).unwrap();
        let parse = t0.elapsed();
        let mut e = Engine::new(48_000.0);
        let t1 = Instant::now();
        e.load_project(&q);
        let load = t1.elapsed();
        println!(
            "SCALE tracks={t} notes={} json={} KB parse={:.2} ms compile={:.2} ms",
            t * n,
            json.len() / 1024,
            parse.as_secs_f64() * 1e3,
            load.as_secs_f64() * 1e3
        );
    }
}
