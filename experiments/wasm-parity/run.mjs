// Spike 002: does the wasm32 engine produce the same audio as the native engine,
// and how much of the AudioWorklet time budget does it use in V8?
//
//   node experiments/wasm-parity/run.mjs <engine_wasm.wasm> <native.f32>
import fs from 'node:fs';

const [wasmPath, nativePath] = process.argv.slice(2);
const bytes = fs.readFileSync(wasmPath);
const t0 = performance.now();
const { instance } = await WebAssembly.instantiate(bytes, {});
const tInst = performance.now() - t0;
const x = instance.exports;
const mem = () => new Float32Array(x.memory.buffer);

function makeEngine(sr) {
  x.cl_init(sr);
  x.cl_add_test_tone(0);
  return x;
}

// ---- 1. parity with native render (demo project, 8 bars, 48 kHz, block 128) ----
const sr = 48000, block = 128;
makeEngine(sr);
x.cl_load_demo(1, 8);
x.cl_command(1, 0, 0, 0); // Play
const native = new Float32Array(fs.readFileSync(nativePath).buffer.slice(0));
const frames = native.length / 2;
let maxDiff = 0, diffCount = 0, firstDiff = -1;
for (let i = 0; i < frames; i += block) {
  const n = Math.min(block, frames - i);
  x.cl_process(n);
  const L = new Float32Array(x.memory.buffer, x.cl_out_l(), n);
  const R = new Float32Array(x.memory.buffer, x.cl_out_r(), n);
  for (let k = 0; k < n; k++) {
    const dl = Math.abs(L[k] - native[2 * (i + k)]);
    const dr = Math.abs(R[k] - native[2 * (i + k) + 1]);
    const d = Math.max(dl, dr);
    if (d > 0) { diffCount++; if (firstDiff < 0) firstDiff = i + k; }
    maxDiff = Math.max(maxDiff, d);
  }
}
console.log(`parity: ${frames} frames compared, ${diffCount} differing samples, max |diff| = ${maxDiff}${firstDiff >= 0 ? `, first at frame ${firstDiff}` : ''}`);
if (diffCount > 0) process.exitCode = 1; // CI gate: native and wasm must be bit-identical

// ---- 1b. i16 / stereo sample ABI (#8) ----
{
  x.cl_init(48000);
  const n = 4800, pcm = new Int16Array(2 * n);
  for (let i = 0; i < n; i++) pcm[2 * i] = 8000; // left only
  const p = x.cl_alloc(pcm.byteLength);
  new Int16Array(x.memory.buffer, p, pcm.length).set(pcm);
  const ok = x.cl_add_sample_i16(7, p, pcm.length, 2, 48000);
  const bad = x.cl_add_sample_i16(8, p, pcm.length - 1, 2, 48000);
  x.cl_free(p, pcm.byteLength);
  const mem = x.cl_sample_memory_bytes();
  console.log(`i16 stereo sample: rc=${ok} (expect 0), odd length rc=${bad} (expect -1), memory=${mem} bytes (expect ${pcm.byteLength})`);
  if (ok !== 0 || bad !== -1 || mem !== pcm.byteLength) process.exitCode = 1;
}

// ---- 2. JSON project load path ----
const json = JSON.stringify({ tempo_bpm: 100, tracks: [{ name: 'p', instrument: { kind: 'synth', wave: 'saw', cutoff_hz: 2000, resonance: 0.2, attack_s: 0.01, decay_s: 0.2, sustain: 0.6, release_s: 0.3 }, clips: [{ start_beat: 0, notes: [{ start: 0, length: 1, pitch: 60 }, { start: 1, length: 1, pitch: 64 }] }] }] });
const enc = new TextEncoder().encode(json);
const p = x.cl_alloc(enc.length);
new Uint8Array(x.memory.buffer, p, enc.length).set(enc);
let t = performance.now();
const rc = x.cl_load_project(p, enc.length);
console.log(`json load rc=${rc} (${enc.length} bytes) in ${(performance.now() - t).toFixed(2)} ms; bad json rc=${x.cl_load_project(p, 3)}`);
x.cl_free(p, enc.length);

// ---- 3. CPU cost per 128-frame block vs polyphony (AudioWorklet render quantum) ----
const budgetUs = (block / sr) * 1e6;
console.log(`wasm instantiate ${tInst.toFixed(1)} ms, module ${bytes.length} bytes; budget ${budgetUs.toFixed(0)} µs / block`);
for (const voices of [8, 16, 32, 64]) {
  makeEngine(sr);
  x.cl_command(1, 0, 0, 0);
  // empty project + live notes from 'track 0' needs a track: load a 1-track synth project
  const pj = new TextEncoder().encode(JSON.stringify({ tempo_bpm: 120, tracks: [{ name: 's', instrument: { kind: 'synth', wave: 'saw', cutoff_hz: 3000, resonance: 0.3, attack_s: 0.01, decay_s: 0.2, sustain: 0.7, release_s: 0.2 }, clips: [] }] }));
  const pp = x.cl_alloc(pj.length); new Uint8Array(x.memory.buffer, pp, pj.length).set(pj); x.cl_load_project(pp, pj.length);
  for (let v = 0; v < voices; v++) x.cl_command(4, 0, 36 + v, 90); // NoteOn
  for (let w = 0; w < 200; w++) x.cl_process(block); // warm-up JIT
  const N = 2000; let worst = 0;
  const s = performance.now();
  for (let b = 0; b < N; b++) { const a = performance.now(); x.cl_process(block); worst = Math.max(worst, performance.now() - a); }
  const avg = ((performance.now() - s) / N) * 1000;
  console.log(`voices=${String(voices).padStart(2)} active=${x.cl_active_voices()} avg ${avg.toFixed(1)} µs (${((100 * avg) / budgetUs).toFixed(1)}% of budget) worst ${(worst * 1000).toFixed(0)} µs`);
}
