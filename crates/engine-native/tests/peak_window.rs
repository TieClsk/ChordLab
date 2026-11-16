//! #48: the desktop status reports the slowest callback of the last second as well as
//! the slowest of the whole session, so one-off spikes can be told apart from steady load.
use engine_native::PeakWindow;

#[test]
fn reports_the_peak_of_each_completed_window() {
    let mut w = PeakWindow::new(48_000);
    assert_eq!(w.recent_ns(), 0);
    for _ in 0..99 {
        w.record(480, 100_000);
    }
    w.record(480, 900_000); // spike in the first second
    assert_eq!(w.recent_ns(), 900_000, "window of 100 x 480 frames = 1 s completed");
    for _ in 0..100 {
        w.record(480, 120_000);
    }
    assert_eq!(w.recent_ns(), 120_000, "the old spike ages out after one second");
    assert_eq!(w.session_ns(), 900_000, "but stays in the session peak");
}

#[test]
fn partial_window_shows_the_peak_so_far_if_higher() {
    let mut w = PeakWindow::new(48_000);
    w.record(480, 50_000);
    assert_eq!(w.recent_ns(), 50_000);
}

#[test]
fn record_is_cheap_and_bounded() {
    let mut w = PeakWindow::new(44_100);
    for i in 0..1_000_000u64 {
        w.record(128, i % 1000);
    }
    assert!(w.session_ns() < 1000);
}
