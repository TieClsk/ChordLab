//! Native host for the engine.
//!
//! Threading model (desktop):
//!   UI / Tauri command thread --(rtrb SPSC, [u32;4] commands)--> audio callback (cpal)
//!   audio callback --(atomics)--> status (position, voices, callback timing)
//! Project / sample loads are prepared off the audio thread and swapped in through a
//! second SPSC queue of boxed engines, so the callback never allocates or blocks.

/// Tracks the slowest audio callback per window (about one second of audio) and over
/// the whole session (#48). Plain integers, no allocation: safe on the audio thread.
pub struct PeakWindow {
    window_frames: u64,
    frames: u64,
    current_ns: u64,
    last_ns: u64,
    session_ns: u64,
}

impl PeakWindow {
    pub fn new(window_frames: u64) -> Self {
        PeakWindow {
            window_frames: window_frames.max(1),
            frames: 0,
            current_ns: 0,
            last_ns: 0,
            session_ns: 0,
        }
    }

    #[inline]
    pub fn record(&mut self, frames: u64, ns: u64) {
        self.current_ns = self.current_ns.max(ns);
        self.session_ns = self.session_ns.max(ns);
        self.frames += frames;
        if self.frames >= self.window_frames {
            self.last_ns = self.current_ns;
            self.current_ns = 0;
            self.frames = 0;
        }
    }

    /// Peak of the last completed window, or of the current one if that is higher.
    pub fn recent_ns(&self) -> u64 {
        self.last_ns.max(self.current_ns)
    }

    pub fn session_ns(&self) -> u64 {
        self.session_ns
    }
}

#[cfg(feature = "realtime")]
pub mod realtime {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use engine_core::{Command, Engine};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;
    use std::time::Instant;

    #[derive(Default)]
    pub struct Status {
        pub position: AtomicU64,
        pub voices: AtomicU64,
        pub callbacks: AtomicU64,
        pub max_callback_ns: AtomicU64,
        /// slowest callback in roughly the last second (#48)
        pub recent_max_callback_ns: AtomicU64,
        pub last_frames: AtomicU64,
    }

    pub struct Host {
        cmd_tx: rtrb::Producer<[u32; 4]>,
        swap_tx: rtrb::Producer<Box<Engine>>,
        trash_rx: rtrb::Consumer<Box<Engine>>,
        pub status: Arc<Status>,
        pub sample_rate: u32,
        pub device_name: String,
        /// dropping this stops the audio thread (and the stream it owns)
        _shutdown: std::sync::mpsc::Sender<()>,
    }

    impl Host {
        /// `cpal::Stream` is `!Send` on several platforms, so a dedicated thread
        /// creates and owns it for the app's lifetime; only Send handles come back.
        pub fn start() -> Result<Host, String> {
            let (ready_tx, ready_rx) = std::sync::mpsc::channel();
            let (shutdown_tx, shutdown_rx) = std::sync::mpsc::channel::<()>();
            std::thread::Builder::new()
                .name("chordlab-audio-host".into())
                .spawn(move || match Self::open() {
                    Ok((parts, stream)) => {
                        let _ = ready_tx.send(Ok(parts));
                        let _ = shutdown_rx.recv(); // park until the Host is dropped
                        drop(stream);
                    }
                    Err(e) => {
                        let _ = ready_tx.send(Err(e));
                    }
                })
                .map_err(|e| e.to_string())?;
            let (cmd_tx, swap_tx, trash_rx, status, sample_rate, device_name) =
                ready_rx.recv().map_err(|e| e.to_string())??;
            Ok(Host {
                cmd_tx,
                swap_tx,
                trash_rx,
                status,
                sample_rate,
                device_name,
                _shutdown: shutdown_tx,
            })
        }

        #[allow(clippy::type_complexity)]
        fn open() -> Result<
            (
                (
                    rtrb::Producer<[u32; 4]>,
                    rtrb::Producer<Box<Engine>>,
                    rtrb::Consumer<Box<Engine>>,
                    Arc<Status>,
                    u32,
                    String,
                ),
                cpal::Stream,
            ),
            String,
        > {
            let host = cpal::default_host();
            let device = host.default_output_device().ok_or("no output device")?;
            let device_name = device.name().unwrap_or_default();
            let cfg = device.default_output_config().map_err(|e| e.to_string())?;
            let sample_rate = cfg.sample_rate().0;
            let channels = cfg.channels() as usize;
            let (cmd_tx, mut cmd_rx) = rtrb::RingBuffer::<[u32; 4]>::new(4096);
            let (swap_tx, mut swap_rx) = rtrb::RingBuffer::<Box<Engine>>::new(4);
            let (mut trash_tx, trash_rx) = rtrb::RingBuffer::<Box<Engine>>::new(4);
            let status = Arc::new(Status::default());
            let st = status.clone();
            let mut engine = Box::new(Engine::new(sample_rate as f32));
            let mut peaks = crate::PeakWindow::new(sample_rate as u64);
            let mut l = vec![0.0f32; 8192];
            let mut r = vec![0.0f32; 8192];
            let stream = device
                .build_output_stream(
                    &cfg.into(),
                    move |out: &mut [f32], _| {
                        let t = Instant::now();
                        if let Ok(new) = swap_rx.pop() {
                            // old engine is handed back for deallocation off-thread
                            let old = std::mem::replace(&mut engine, new);
                            let _ = trash_tx.push(old);
                        }
                        while let Ok(w) = cmd_rx.pop() {
                            if let Some(c) = Command::decode(w) {
                                engine.apply(c);
                            }
                        }
                        let frames = (out.len() / channels).min(l.len());
                        engine.process(&mut l[..frames], &mut r[..frames]);
                        for i in 0..frames {
                            out[i * channels] = l[i];
                            if channels > 1 {
                                out[i * channels + 1] = r[i];
                            }
                        }
                        st.position.store(engine.position(), Ordering::Relaxed);
                        st.voices
                            .store(engine.active_voices() as u64, Ordering::Relaxed);
                        st.callbacks.fetch_add(1, Ordering::Relaxed);
                        st.last_frames.store(frames as u64, Ordering::Relaxed);
                        peaks.record(frames as u64, t.elapsed().as_nanos() as u64);
                        st.max_callback_ns
                            .store(peaks.session_ns(), Ordering::Relaxed);
                        st.recent_max_callback_ns
                            .store(peaks.recent_ns(), Ordering::Relaxed);
                    },
                    |e| eprintln!("audio stream error: {e}"),
                    None,
                )
                .map_err(|e| e.to_string())?;
            stream.play().map_err(|e| e.to_string())?;
            Ok((
                (cmd_tx, swap_tx, trash_rx, status, sample_rate, device_name),
                stream,
            ))
        }

        pub fn command(&mut self, c: Command) -> bool {
            self.cmd_tx.push(c.encode()).is_ok()
        }
        pub fn command_raw(&mut self, w: [u32; 4]) -> bool {
            self.cmd_tx.push(w).is_ok()
        }

        /// Build a fully-loaded engine off the audio thread and swap it in.
        pub fn replace_engine(&mut self, e: Engine) -> bool {
            // free engines the audio thread handed back (deallocation happens here, not in the callback)
            while self.trash_rx.pop().is_ok() {}
            self.swap_tx.push(Box::new(e)).is_ok()
        }
    }
}
