//! Plain-ABI wasm exports for the AudioWorklet host.
//!
//! Why no wasm-bindgen: AudioWorkletGlobalScope lacks TextDecoder/fetch, and the
//! generated glue assumes them. A handful of `extern "C"` functions with pointers
//! into linear memory is enough and keeps the module tiny.
//!
//! The AudioWorklet is single-threaded, so a single global engine is safe.
#![allow(static_mut_refs)]

use engine_core::{Command, Engine, Project};

pub const MAX_BLOCK: usize = 4096;

struct State {
    engine: Engine,
    out_l: [f32; MAX_BLOCK],
    out_r: [f32; MAX_BLOCK],
}

static mut STATE: Option<State> = None;

fn st() -> &'static mut State {
    unsafe { STATE.as_mut().expect("engine_init not called") }
}

/// Allocate `len` bytes in linear memory for the host to fill.
#[no_mangle]
pub extern "C" fn cl_alloc(len: usize) -> *mut u8 {
    let mut v = Vec::<u8>::with_capacity(len.max(1));
    let p = v.as_mut_ptr();
    core::mem::forget(v);
    p
}

/// # Safety
/// `ptr`/`len` must come from a previous `cl_alloc(len)` call.
#[no_mangle]
pub unsafe extern "C" fn cl_free(ptr: *mut u8, len: usize) {
    drop(Vec::from_raw_parts(ptr, 0, len.max(1)));
}

#[no_mangle]
pub extern "C" fn cl_init(sample_rate: f32) {
    unsafe {
        STATE = Some(State {
            engine: Engine::new(sample_rate),
            out_l: [0.0; MAX_BLOCK],
            out_r: [0.0; MAX_BLOCK],
        });
    }
}

/// Load a project from UTF-8 JSON at `ptr`. Returns 0 on success, -1 on parse error.
///
/// # Safety
/// `ptr..ptr+len` must be readable linear memory (e.g. from `cl_alloc`).
#[no_mangle]
pub unsafe extern "C" fn cl_load_project(ptr: *const u8, len: usize) -> i32 {
    let bytes = core::slice::from_raw_parts(ptr, len);
    match serde_json::from_slice::<Project>(bytes) {
        Ok(p) => {
            st().engine.load_project(&p);
            0
        }
        Err(_) => -1,
    }
}

/// Register a mono f32 sample. Data is copied.
///
/// # Safety
/// `ptr` must point to `len` initialised, aligned `f32` values in linear memory.
#[no_mangle]
pub unsafe extern "C" fn cl_add_sample(id: u32, ptr: *const f32, len: usize, sample_rate: f32) {
    let data = core::slice::from_raw_parts(ptr, len).to_vec();
    st().engine.add_sample(id, data, sample_rate);
}

/// Register an i16 sample, mono (`channels` = 1) or interleaved stereo (2). Data is
/// copied. Returns 0 on success, -1 for invalid input (#8).
///
/// # Safety
/// `ptr` must point to `len` initialised, aligned `i16` values in linear memory.
#[no_mangle]
pub unsafe extern "C" fn cl_add_sample_i16(
    id: u32,
    ptr: *const i16,
    len: usize,
    channels: u32,
    sample_rate: f32,
) -> i32 {
    let data = core::slice::from_raw_parts(ptr, len).to_vec();
    if st()
        .engine
        .add_sample_i16(id, data, channels as u8, sample_rate)
    {
        0
    } else {
        -1
    }
}

/// Bytes of sample data held by the engine.
#[no_mangle]
pub extern "C" fn cl_sample_memory_bytes() -> u32 {
    st().engine.sample_memory_bytes() as u32
}

/// Apply one 16-byte command (see `engine_core::Command::encode`). Returns 0 if valid.
#[no_mangle]
pub extern "C" fn cl_command(a: u32, b: u32, c: u32, d: u32) -> i32 {
    match Command::decode([a, b, c, d]) {
        Some(cmd) => {
            st().engine.apply(cmd);
            0
        }
        None => -1,
    }
}

/// Render `frames` (≤ MAX_BLOCK) into the output buffers.
#[no_mangle]
pub extern "C" fn cl_process(frames: usize) {
    let s = st();
    let n = frames.min(MAX_BLOCK);
    let State {
        engine,
        out_l,
        out_r,
    } = s;
    engine.process(&mut out_l[..n], &mut out_r[..n]);
}

#[no_mangle]
pub extern "C" fn cl_out_l() -> *const f32 {
    st().out_l.as_ptr()
}
#[no_mangle]
pub extern "C" fn cl_out_r() -> *const f32 {
    st().out_r.as_ptr()
}
#[no_mangle]
pub extern "C" fn cl_position() -> f64 {
    st().engine.position() as f64
}
#[no_mangle]
pub extern "C" fn cl_active_voices() -> u32 {
    st().engine.active_voices() as u32
}

/// Convenience for tests/spikes: load the built-in demo project.
#[no_mangle]
pub extern "C" fn cl_load_demo(with_sampler: u32, bars: u32) {
    let p = engine_core::demo_project(with_sampler != 0, bars);
    st().engine.load_project(&p);
}
#[no_mangle]
pub extern "C" fn cl_add_test_tone(id: u32) {
    let sr = st().engine.sample_rate();
    let d = engine_core::test_tone_sample(sr);
    st().engine.add_sample(id, d, sr);
}
