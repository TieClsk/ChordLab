//! Real-time engine: sample-accurate event scheduling, voice pool, mixer.
//! `process` performs no heap allocation; `load_project` / `add_sample` may allocate
//! and must be called off the audio thread (native) or between blocks (wasm).

use crate::dsp::*;
use crate::model::{Instrument, Project, Wave};

pub const MAX_VOICES: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Event {
    pub frame: u64,
    pub track: u16,
    pub on: bool,
    pub pitch: u8,
    pub velocity: u8,
}

/// Fixed-size command for lock-free UI -> audio thread transport.
/// Encoded as 4 x u32 (16 bytes) so it fits a SharedArrayBuffer / rtrb ring buffer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Command {
    Play,
    Stop,
    Seek { frame: u64 },
    NoteOn { track: u16, pitch: u8, velocity: u8 },
    NoteOff { track: u16, pitch: u8 },
    SetTrackGain { track: u16, gain: f32 },
    SetTrackPan { track: u16, pan: f32 },
    SetMute { track: u16, mute: bool },
    SetLoop { start: u32, end: u32 },
}

impl Command {
    pub fn encode(self) -> [u32; 4] {
        use Command::*;
        match self {
            Play => [1, 0, 0, 0],
            Stop => [2, 0, 0, 0],
            Seek { frame } => [3, frame as u32, (frame >> 32) as u32, 0],
            NoteOn {
                track,
                pitch,
                velocity,
            } => [4, track as u32, pitch as u32, velocity as u32],
            NoteOff { track, pitch } => [5, track as u32, pitch as u32, 0],
            SetTrackGain { track, gain } => [6, track as u32, gain.to_bits(), 0],
            SetTrackPan { track, pan } => [7, track as u32, pan.to_bits(), 0],
            SetMute { track, mute } => [8, track as u32, mute as u32, 0],
            SetLoop { start, end } => [9, start, end, 0],
        }
    }
    pub fn decode(w: [u32; 4]) -> Option<Command> {
        use Command::*;
        Some(match w[0] {
            1 => Play,
            2 => Stop,
            3 => Seek {
                frame: w[1] as u64 | ((w[2] as u64) << 32),
            },
            4 => NoteOn {
                track: w[1] as u16,
                pitch: w[2] as u8,
                velocity: w[3] as u8,
            },
            5 => NoteOff {
                track: w[1] as u16,
                pitch: w[2] as u8,
            },
            6 => SetTrackGain {
                track: w[1] as u16,
                gain: f32::from_bits(w[2]),
            },
            7 => SetTrackPan {
                track: w[1] as u16,
                pan: f32::from_bits(w[2]),
            },
            8 => SetMute {
                track: w[1] as u16,
                mute: w[2] != 0,
            },
            9 => SetLoop {
                start: w[1],
                end: w[2],
            },
            _ => return None,
        })
    }
}

pub struct Sample {
    pub data: Vec<f32>,
    pub sample_rate: f32,
}

struct TrackState {
    instrument: Instrument,
    gain: f32,
    pan_l: f32,
    pan_r: f32,
    mute: bool,
    send: f32,
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Synth,
    Sampler,
    Drum,
}

#[derive(Clone, Copy)]
struct Voice {
    active: bool,
    kind: Kind,
    track: u16,
    pitch: u8,
    age: u64,
    amp: f32,
    // synth
    osc: Osc,
    osc2: Osc,
    svf: Svf,
    env: Adsr,
    dt: f32,
    fcoef: f32,
    q: f32,
    wave: Wave,
    // sampler
    sample: usize,
    spos: f64,
    sstep: f64,
    // drum
    drum: u8,
    t: u32,
    level: f32,
    decay: f32,
    noise: Noise,
    hp: f32,
}

impl Default for Voice {
    fn default() -> Self {
        Voice {
            active: false,
            kind: Kind::Synth,
            track: 0,
            pitch: 0,
            age: 0,
            amp: 0.0,
            osc: Osc::default(),
            osc2: Osc::default(),
            svf: Svf::default(),
            env: Adsr::default(),
            dt: 0.0,
            fcoef: 0.0,
            q: 1.0,
            wave: Wave::Saw,
            sample: 0,
            spos: 0.0,
            sstep: 1.0,
            drum: 0,
            t: 0,
            level: 0.0,
            decay: 0.0,
            noise: Noise(0x1234_5678),
            hp: 0.0,
        }
    }
}

pub struct Engine {
    sr: f32,
    midi: [f32; 128],
    tracks: Vec<TrackState>,
    events: Vec<Event>,
    cursor: usize,
    pos: u64,
    playing: bool,
    loop_range: Option<(u64, u64)>,
    voices: [Voice; MAX_VOICES],
    voice_clock: u64,
    samples: Vec<Option<Sample>>,
    delay: Delay,
    master: f32,
    /// frames rendered since creation (for metering / diagnostics)
    pub frames_rendered: u64,
}

impl Engine {
    pub fn new(sample_rate: f32) -> Self {
        Engine {
            sr: sample_rate,
            midi: midi_table(),
            tracks: Vec::new(),
            events: Vec::new(),
            cursor: 0,
            pos: 0,
            playing: false,
            loop_range: None,
            voices: [Voice::default(); MAX_VOICES],
            voice_clock: 0,
            samples: Vec::new(),
            delay: Delay::new((sample_rate * 0.375) as usize),
            master: 0.8,
            frames_rendered: 0,
        }
    }

    pub fn sample_rate(&self) -> f32 {
        self.sr
    }
    pub fn position(&self) -> u64 {
        self.pos
    }
    pub fn is_playing(&self) -> bool {
        self.playing
    }
    pub fn active_voices(&self) -> usize {
        self.voices.iter().filter(|v| v.active).count()
    }

    pub fn add_sample(&mut self, id: u32, data: Vec<f32>, sample_rate: f32) {
        let id = id as usize;
        if self.samples.len() <= id {
            self.samples.resize_with(id + 1, || None);
        }
        self.samples[id] = Some(Sample { data, sample_rate });
    }

    /// Compile the project into a sorted event list. Allocates.
    pub fn load_project(&mut self, p: &Project) {
        let sr = self.sr as f64;
        self.master = p.master_gain;
        self.tracks = p
            .tracks
            .iter()
            .map(|t| {
                let (l, r) = pan_gains(t.pan);
                TrackState {
                    instrument: t.instrument.clone(),
                    gain: t.gain,
                    pan_l: l,
                    pan_r: r,
                    mute: t.mute,
                    send: t.delay_send,
                }
            })
            .collect();
        let mut ev = Vec::new();
        for (ti, t) in p.tracks.iter().enumerate() {
            for c in &t.clips {
                for n in &c.notes {
                    let on = p.beats_to_frames(c.start_beat + n.start, sr);
                    let off = p
                        .beats_to_frames(c.start_beat + n.start + n.length, sr)
                        .max(on + 1);
                    ev.push(Event {
                        frame: on,
                        track: ti as u16,
                        on: true,
                        pitch: n.pitch,
                        velocity: n.velocity,
                    });
                    ev.push(Event {
                        frame: off,
                        track: ti as u16,
                        on: false,
                        pitch: n.pitch,
                        velocity: 0,
                    });
                }
            }
        }
        // note-offs before note-ons at the same frame so re-triggered notes work
        ev.sort_by(|a, b| {
            a.frame
                .cmp(&b.frame)
                .then(a.on.cmp(&b.on))
                .then(a.track.cmp(&b.track))
                .then(a.pitch.cmp(&b.pitch))
        });
        self.events = ev;
        self.seek(self.pos);
    }

    pub fn seek(&mut self, frame: u64) {
        self.pos = frame;
        self.cursor = self.events.partition_point(|e| e.frame < frame);
        for v in self.voices.iter_mut() {
            if v.kind != Kind::Drum {
                v.env.gate_off();
            }
        }
    }

    pub fn apply(&mut self, cmd: Command) {
        match cmd {
            Command::Play => self.playing = true,
            Command::Stop => {
                self.playing = false;
                self.all_notes_off();
            }
            Command::Seek { frame } => self.seek(frame),
            Command::NoteOn {
                track,
                pitch,
                velocity,
            } => self.note_on(track, pitch, velocity),
            Command::NoteOff { track, pitch } => self.note_off(track, pitch),
            Command::SetTrackGain { track, gain } => {
                if let Some(t) = self.tracks.get_mut(track as usize) {
                    t.gain = gain
                }
            }
            Command::SetTrackPan { track, pan } => {
                if let Some(t) = self.tracks.get_mut(track as usize) {
                    let (l, r) = pan_gains(pan);
                    t.pan_l = l;
                    t.pan_r = r;
                }
            }
            Command::SetMute { track, mute } => {
                if let Some(t) = self.tracks.get_mut(track as usize) {
                    t.mute = mute
                }
            }
            Command::SetLoop { start, end } => {
                self.loop_range = if end > start {
                    Some((start as u64, end as u64))
                } else {
                    None
                }
            }
        }
    }

    pub fn all_notes_off(&mut self) {
        for v in self.voices.iter_mut() {
            v.env.gate_off();
        }
    }

    fn alloc_voice(&mut self) -> usize {
        if let Some(i) = self.voices.iter().position(|v| !v.active) {
            return i;
        }
        // steal the oldest voice
        let mut best = 0;
        for i in 1..MAX_VOICES {
            if self.voices[i].age < self.voices[best].age {
                best = i;
            }
        }
        best
    }

    #[allow(clippy::field_reassign_with_default)]
    pub fn note_on(&mut self, track: u16, pitch: u8, velocity: u8) {
        let Some(ts) = self.tracks.get(track as usize) else {
            return;
        };
        let pitch = pitch.min(127);
        let vel = velocity as f32 / 127.0;
        let freq = self.midi[pitch as usize];
        let sr = self.sr;
        let mut v = Voice::default();
        v.active = true;
        v.track = track;
        v.pitch = pitch;
        self.voice_clock += 1;
        v.age = self.voice_clock;
        v.amp = vel * vel;
        match &ts.instrument {
            Instrument::Synth(p) => {
                v.kind = Kind::Synth;
                v.dt = freq / sr;
                v.wave = p.wave;
                v.fcoef = svf_coef(p.cutoff_hz * (0.6 + 0.8 * vel), sr);
                v.q = (1.0 - p.resonance.clamp(0.0, 0.95)) * 2.0;
                v.osc2.phase = 0.37;
                v.env.set(p.attack_s, p.decay_s, p.sustain, p.release_s, sr);
                v.env.gate_on();
            }
            Instrument::Sampler { zones, release_s } => {
                let Some(z) = zones.iter().find(|z| {
                    pitch >= z.lo_note
                        && pitch <= z.hi_note
                        && velocity >= z.lo_vel
                        && velocity <= z.hi_vel
                }) else {
                    return;
                };
                let Some(Some(s)) = self.samples.get(z.sample_id as usize) else {
                    return;
                };
                v.kind = Kind::Sampler;
                // velocity already selects the layer; keep a gentler curve on top
                v.amp = (0.4 + 0.6 * vel) * z.gain;
                v.sample = z.sample_id as usize;
                v.spos = 0.0;
                v.sstep =
                    (freq / self.midi[z.root_note as usize]) as f64 * (s.sample_rate / sr) as f64;
                v.env.set(0.002, 0.01, 1.0, *release_s, sr);
                v.env.gate_on();
            }
            Instrument::Drums => {
                v.kind = Kind::Drum;
                v.drum = pitch;
                v.level = 1.0;
                let decay_s = match pitch {
                    36 => 0.45,
                    38 | 40 => 0.18,
                    _ => 0.06,
                };
                v.decay = 1.0 - 1.0 / (decay_s * sr / 4.6);
                v.noise =
                    Noise(0x9E37_79B9 ^ (self.voice_clock as u32).wrapping_mul(2_654_435_761));
            }
        }
        // Retrigger: a pitch that is still held on this track reuses its voice, so
        // repeated NoteOns (key auto-repeat, overlapping clips) never stack (#42).
        // Voices already in their release tail are left to ring out.
        let held = self.voices.iter().position(|o| {
            o.active
                && o.track == track
                && o.pitch == pitch
                && o.kind != Kind::Drum
                && o.env.stage != Stage::Release
        });
        let i = match held {
            Some(i) => {
                // keep phase/filter/envelope level for a click-free retrigger
                let old = self.voices[i];
                v.osc = old.osc;
                v.osc2 = old.osc2;
                v.svf = old.svf;
                v.env.level = old.env.level;
                i
            }
            None => self.alloc_voice(),
        };
        self.voices[i] = v;
    }

    pub fn note_off(&mut self, track: u16, pitch: u8) {
        for v in self.voices.iter_mut() {
            if v.active
                && v.track == track
                && v.pitch == pitch
                && v.kind != Kind::Drum
                && v.env.stage != Stage::Release
            {
                v.env.gate_off();
                break;
            }
        }
    }

    /// Render one block. No allocation.
    pub fn process(&mut self, out_l: &mut [f32], out_r: &mut [f32]) {
        let n = out_l.len().min(out_r.len());
        let mut i = 0;
        while i < n {
            // dispatch all events due at the current frame
            let mut seg_end = n;
            if self.playing {
                while let Some(e) = self.events.get(self.cursor) {
                    if e.frame > self.pos {
                        break;
                    }
                    let e = *e;
                    self.cursor += 1;
                    if e.on {
                        self.note_on(e.track, e.pitch, e.velocity)
                    } else {
                        self.note_off(e.track, e.pitch)
                    }
                }
                if let Some(e) = self.events.get(self.cursor) {
                    seg_end = seg_end.min(i + (e.frame - self.pos) as usize);
                }
                if let Some((_, le)) = self.loop_range {
                    if self.pos < le {
                        seg_end = seg_end.min(i + (le - self.pos) as usize);
                    }
                }
            }
            self.render_segment(&mut out_l[i..seg_end], &mut out_r[i..seg_end]);
            let len = (seg_end - i) as u64;
            if self.playing {
                self.pos += len;
                if let Some((ls, le)) = self.loop_range {
                    if self.pos >= le {
                        self.seek(ls);
                    }
                }
            }
            i = seg_end;
        }
        self.frames_rendered += n as u64;
    }

    fn render_segment(&mut self, out_l: &mut [f32], out_r: &mut [f32]) {
        let sr = self.sr;
        for k in 0..out_l.len() {
            let (mut l, mut r, mut sl, mut sr_send) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
            for v in self.voices.iter_mut() {
                if !v.active {
                    continue;
                }
                let ts = &self.tracks[v.track as usize];
                let s = match v.kind {
                    Kind::Synth => {
                        let raw = match v.wave {
                            Wave::Saw => 0.5 * (v.osc.saw(v.dt) + v.osc2.saw(v.dt * 1.003)),
                            Wave::Square => v.osc.square(v.dt),
                        };
                        let y = v.svf.lowpass(raw, v.fcoef, v.q);
                        let e = v.env.tick();
                        if v.env.stage == Stage::Idle {
                            v.active = false;
                        }
                        y * e * 0.3
                    }
                    Kind::Sampler => {
                        let Some(Some(smp)) = self.samples.get(v.sample) else {
                            v.active = false;
                            continue;
                        };
                        let idx = v.spos as usize;
                        if idx + 1 >= smp.data.len() {
                            v.active = false;
                            continue;
                        }
                        let frac = (v.spos - idx as f64) as f32;
                        let y = smp.data[idx] + (smp.data[idx + 1] - smp.data[idx]) * frac;
                        v.spos += v.sstep;
                        let e = v.env.tick();
                        if v.env.stage == Stage::Idle {
                            v.active = false;
                        }
                        y * e
                    }
                    Kind::Drum => {
                        v.t += 1;
                        let t = v.t as f32 / sr;
                        let y = match v.drum {
                            36 => {
                                // pitch sweep 150 -> 45 Hz over ~80 ms
                                let sweep = if t < 0.08 { 1.0 - t / 0.08 } else { 0.0 };
                                let f = 45.0 + 105.0 * sweep * sweep;
                                v.osc.sine(f / sr) * 0.9
                            }
                            38 | 40 => 0.6 * v.noise.tick() + 0.4 * v.osc.sine(180.0 / sr),
                            _ => {
                                let x = v.noise.tick();
                                v.hp = v.hp + 0.2 * (x - v.hp);
                                (x - v.hp) * 0.5
                            }
                        };
                        v.level *= v.decay;
                        if v.level < 0.0005 {
                            v.active = false;
                        }
                        y * v.level
                    }
                };
                if ts.mute {
                    continue;
                }
                let g = s * v.amp * ts.gain;
                l += g * ts.pan_l;
                r += g * ts.pan_r;
                sl += g * ts.pan_l * ts.send;
                sr_send += g * ts.pan_r * ts.send;
            }
            let (dl, dr) = self.delay.process(sl, sr_send);
            out_l[k] = soft_clip((l + dl) * self.master);
            out_r[k] = soft_clip((r + dr) * self.master);
        }
    }
}

/// Render a whole project offline (export). Returns (left, right).
pub fn render_offline(
    p: &Project,
    samples: Vec<(u32, Vec<f32>, f32)>,
    sample_rate: f32,
    tail_s: f32,
    block: usize,
) -> (Vec<f32>, Vec<f32>) {
    let mut e = Engine::new(sample_rate);
    for (id, d, sr) in samples {
        e.add_sample(id, d, sr);
    }
    e.load_project(p);
    e.apply(Command::Play);
    let total = (p.beats_to_frames(p.length_beats(), sample_rate as f64) as f32
        + tail_s * sample_rate) as usize;
    let mut l = vec![0.0; total];
    let mut r = vec![0.0; total];
    let mut i = 0;
    while i < total {
        let end = (i + block).min(total);
        let (bl, br) = (&mut l[i..end], &mut r[i..end]);
        e.process(bl, br);
        i = end;
    }
    (l, r)
}
