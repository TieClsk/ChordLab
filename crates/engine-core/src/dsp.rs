//! Deterministic DSP building blocks.
//! Only + - * / and comparisons are used on the audio path (no libm calls), so the
//! native and wasm32 builds produce bit-identical output for the same input.

/// Flushes values below -300 dB to exact zero. Decaying feedback (delay, filters)
/// would otherwise reach subnormal floats, which are many times slower on x86 (#48).
/// Comparison-only, so native and wasm32 stay bit-identical.
#[inline]
pub fn flush(x: f32) -> f32 {
    if x.abs() < 1e-15 {
        0.0
    } else {
        x
    }
}

/// MIDI note -> Hz table built with repeated multiplication (no powf).
pub fn midi_table() -> [f32; 128] {
    const SEMI: f64 = 1.059_463_094_359_295_3; // 2^(1/12)
    let mut t = [0.0f64; 128];
    t[69] = 440.0;
    for i in 70..128 {
        t[i] = t[i - 1] * SEMI;
    }
    for i in (0..69).rev() {
        t[i] = t[i + 1] / SEMI;
    }
    let mut out = [0.0f32; 128];
    for i in 0..128 {
        out[i] = t[i] as f32;
    }
    out
}

/// Parabolic sine approximation on phase in [0,1). Max error ~0.1%.
#[inline]
pub fn sin01(phase: f32) -> f32 {
    let x = phase * 2.0 - 1.0; // -1..1  maps to -pi..pi
    let y = 4.0 * x * (1.0 - x.abs());
    // extra precision step
    -(0.225 * (y * y.abs() - y) + y)
}

#[inline]
fn poly_blep(t: f32, dt: f32) -> f32 {
    if t < dt {
        let x = t / dt;
        x + x - x * x - 1.0
    } else if t > 1.0 - dt {
        let x = (t - 1.0) / dt;
        x * x + x + x + 1.0
    } else {
        0.0
    }
}

#[derive(Clone, Copy, Default)]
pub struct Osc {
    pub phase: f32,
}

impl Osc {
    #[inline]
    pub fn saw(&mut self, dt: f32) -> f32 {
        let v = 2.0 * self.phase - 1.0 - poly_blep(self.phase, dt);
        self.advance(dt);
        v
    }
    #[inline]
    pub fn square(&mut self, dt: f32) -> f32 {
        let mut v = if self.phase < 0.5 { 1.0 } else { -1.0 };
        v += poly_blep(self.phase, dt);
        let mut p2 = self.phase + 0.5;
        if p2 >= 1.0 {
            p2 -= 1.0;
        }
        v -= poly_blep(p2, dt);
        self.advance(dt);
        v
    }
    #[inline]
    pub fn sine(&mut self, dt: f32) -> f32 {
        let v = sin01(self.phase);
        self.advance(dt);
        v
    }
    #[inline]
    fn advance(&mut self, dt: f32) {
        self.phase += dt;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
    }
}

/// Chamberlin-style state variable filter (low-pass output), stable up to ~sr/6.
#[derive(Clone, Copy, Default)]
pub struct Svf {
    low: f32,
    band: f32,
}

impl Svf {
    /// `f` = 2*sin(pi*fc/sr) precomputed by `svf_coef`.
    #[inline]
    pub fn lowpass(&mut self, x: f32, f: f32, q: f32) -> f32 {
        self.low = flush(self.low + f * self.band);
        let high = x - self.low - q * self.band;
        self.band = flush(self.band + f * high);
        self.low
    }
}

pub fn svf_coef(cutoff_hz: f32, sr: f32) -> f32 {
    let fc = cutoff_hz.min(sr / 6.0).max(20.0);
    // 2*sin(pi*fc/sr) with our deterministic sine: phase = fc/(2sr)
    2.0 * sin01(fc / (2.0 * sr))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

/// Linear-segment ADSR (deterministic, cheap).
#[derive(Clone, Copy)]
pub struct Adsr {
    pub stage: Stage,
    pub level: f32,
    a_inc: f32,
    d_dec: f32,
    sustain: f32,
    r_dec: f32,
}

impl Default for Adsr {
    fn default() -> Self {
        Adsr {
            stage: Stage::Idle,
            level: 0.0,
            a_inc: 1.0,
            d_dec: 1.0,
            sustain: 1.0,
            r_dec: 1.0,
        }
    }
}

impl Adsr {
    pub fn set(&mut self, a: f32, d: f32, s: f32, r: f32, sr: f32) {
        self.a_inc = 1.0 / (a.max(0.0005) * sr);
        self.d_dec = (1.0 - s) / (d.max(0.0005) * sr);
        self.sustain = s;
        self.r_dec = 1.0 / (r.max(0.0005) * sr);
    }
    pub fn gate_on(&mut self) {
        self.stage = Stage::Attack;
    }
    pub fn gate_off(&mut self) {
        if self.stage != Stage::Idle {
            self.stage = Stage::Release;
        }
    }
    #[inline]
    pub fn tick(&mut self) -> f32 {
        match self.stage {
            Stage::Idle => {}
            Stage::Attack => {
                self.level += self.a_inc;
                if self.level >= 1.0 {
                    self.level = 1.0;
                    self.stage = Stage::Decay;
                }
            }
            Stage::Decay => {
                self.level -= self.d_dec;
                if self.level <= self.sustain {
                    self.level = self.sustain;
                    self.stage = Stage::Sustain;
                }
            }
            Stage::Sustain => {}
            Stage::Release => {
                self.level -= self.r_dec;
                if self.level <= 0.0 {
                    self.level = 0.0;
                    self.stage = Stage::Idle;
                }
            }
        }
        self.level
    }
}

/// xorshift32 noise in [-1, 1]
#[derive(Clone, Copy)]
pub struct Noise(pub u32);
impl Noise {
    #[inline]
    pub fn tick(&mut self) -> f32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        (x as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

/// Soft clipper (rational tanh approximation).
#[inline]
pub fn soft_clip(x: f32) -> f32 {
    let x = x.clamp(-3.0, 3.0);
    x * (27.0 + x * x) / (27.0 + 9.0 * x * x)
}

/// Equal-power-ish pan using a quadratic law (no sqrt/sin).
#[inline]
pub fn pan_gains(pan: f32) -> (f32, f32) {
    let p = (pan.clamp(-1.0, 1.0) + 1.0) * 0.5; // 0..1
    let l = 1.0 - p * p;
    let r = 1.0 - (1.0 - p) * (1.0 - p);
    (l, r)
}

/// Ping-pong feedback delay used as a send effect. The buffer is allocated once for
/// `max_len` frames; the active length can change without allocating (tempo sync).
pub struct Delay {
    buf_l: Vec<f32>,
    buf_r: Vec<f32>,
    len: usize,
    pos: usize,
    pub feedback: f32,
}

impl Delay {
    pub fn new(max_len: usize) -> Self {
        let n = max_len.max(1);
        Delay {
            buf_l: vec![0.0; n],
            buf_r: vec![0.0; n],
            len: n,
            pos: 0,
            feedback: 0.35,
        }
    }

    /// Sets the delay time in frames (clamped to the buffer) and clears old echoes.
    pub fn set_len(&mut self, frames: usize) {
        self.len = frames.clamp(1, self.buf_l.len());
        self.pos = 0;
        self.buf_l.iter_mut().for_each(|x| *x = 0.0);
        self.buf_r.iter_mut().for_each(|x| *x = 0.0);
    }

    pub fn frames(&self) -> usize {
        self.len
    }

    #[inline]
    pub fn process(&mut self, l: f32, r: f32) -> (f32, f32) {
        let (ol, or) = (self.buf_l[self.pos], self.buf_r[self.pos]);
        self.buf_l[self.pos] = flush(l + or * self.feedback);
        self.buf_r[self.pos] = flush(r + ol * self.feedback);
        self.pos += 1;
        if self.pos >= self.len {
            self.pos = 0;
        }
        (ol, or)
    }
}
