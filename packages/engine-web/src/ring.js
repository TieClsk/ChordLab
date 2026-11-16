// Wait-free single-producer / single-consumer ring of 16-byte commands over a
// SharedArrayBuffer. Producer: UI thread. Consumer: AudioWorkletProcessor.
// Layout: Int32[0] = write index, Int32[1] = read index, then capacity * 4 Uint32 slots.
// Mirrors `engine_core::Command::encode` ([u32; 4]).

export const HEADER_BYTES = 8;
export const SLOT_WORDS = 4;

export function createRingBuffer(capacity = 1024) {
  return new SharedArrayBuffer(HEADER_BYTES + capacity * SLOT_WORDS * 4);
}

export class RingWriter {
  constructor(sab) {
    this.idx = new Int32Array(sab, 0, 2);
    this.data = new Uint32Array(sab, HEADER_BYTES);
    this.cap = this.data.length / SLOT_WORDS;
  }
  /** @returns {boolean} false if the ring is full */
  push(a, b = 0, c = 0, d = 0) {
    const w = Atomics.load(this.idx, 0);
    const r = Atomics.load(this.idx, 1);
    const next = (w + 1) % this.cap;
    if (next === r) return false;
    const o = w * SLOT_WORDS;
    this.data[o] = a; this.data[o + 1] = b; this.data[o + 2] = c; this.data[o + 3] = d;
    Atomics.store(this.idx, 0, next); // publish after the payload is written
    return true;
  }
}

export class RingReader {
  constructor(sab) {
    this.idx = new Int32Array(sab, 0, 2);
    this.data = new Uint32Array(sab, HEADER_BYTES);
    this.cap = this.data.length / SLOT_WORDS;
  }
  /** Calls fn(a,b,c,d) for every pending command. Returns the count. */
  drain(fn) {
    let r = Atomics.load(this.idx, 1);
    const w = Atomics.load(this.idx, 0);
    let n = 0;
    while (r !== w) {
      const o = r * SLOT_WORDS;
      fn(this.data[o], this.data[o + 1], this.data[o + 2], this.data[o + 3]);
      r = (r + 1) % this.cap;
      n++;
    }
    Atomics.store(this.idx, 1, r);
    return n;
  }
}

// Float helper so callers can send f32 parameters (gain, pan) as u32 bits.
const f32 = new Float32Array(1);
const u32 = new Uint32Array(f32.buffer);
export function f32bits(x) { f32[0] = x; return u32[0]; }
