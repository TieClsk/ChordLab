//! #48: subnormal floats make x86 CPUs many times slower. The audio path must flush
//! decaying tails (delay feedback, filters) to zero instead of letting them go subnormal.
use engine_core::*;

#[test]
fn delay_tail_never_becomes_subnormal() {
    let mut p = demo_project(false, 2);
    for t in p.tracks.iter_mut() {
        t.delay_send = 0.6;
    }
    let sr = 48_000.0;
    let mut e = Engine::new(sr);
    e.load_project(&p);
    e.apply(Command::Play);
    let (mut l, mut r) = (vec![0.0; 480], vec![0.0; 480]);
    for b in 0..(70 * 100) {
        if b == 500 {
            e.apply(Command::Stop); // let the feedback tail decay for over a minute
        }
        e.process(&mut l, &mut r);
        let bad = l.iter().chain(r.iter()).find(|x| x.is_subnormal());
        assert!(
            bad.is_none(),
            "subnormal output {:e} at {:.2} s",
            bad.unwrap(),
            b as f32 / 100.0
        );
    }
    assert!(
        l.iter().chain(r.iter()).all(|x| *x == 0.0),
        "tail should end in exact silence"
    );
}

#[test]
fn delay_tail_is_still_audible_before_it_is_flushed() {
    // the flush threshold must be far below hearing (-300 dB), not cut echoes short
    let mut p = demo_project(false, 1);
    for t in p.tracks.iter_mut() {
        t.delay_send = 0.6;
    }
    let mut e = Engine::new(48_000.0);
    e.load_project(&p);
    e.apply(Command::Play);
    let (mut l, mut r) = (vec![0.0; 480], vec![0.0; 480]);
    for _ in 0..300 {
        e.process(&mut l, &mut r);
    }
    e.apply(Command::Stop);
    for _ in 0..100 {
        e.process(&mut l, &mut r); // 1 s after stop
    }
    assert!(
        l.iter().any(|x| x.abs() > 1e-4),
        "echoes one second after stop are still audible"
    );
}
