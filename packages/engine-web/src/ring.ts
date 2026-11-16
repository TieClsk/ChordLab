// Wait-free single-producer / single-consumer ring of 16-byte commands over a
// SharedArrayBuffer. Producer: UI thread. Consumer: AudioWorkletProcessor.
// Layout: Int32[0] = write index, Int32[1] = read index, then capacity * 4 Uint32 slots.

const HEADER_BYTES = 8;
const SLOT_WORDS = 4;

export function createRingBuffer(capacity = 1024): SharedArrayBuffer {
  return new SharedArrayBuffer(HEADER_BYTES + capacity * SLOT_WORDS * 4);
}

export class RingWriter {
  private readonly idx: Int32Array;
  private readonly data: Uint32Array;
  private readonly cap: number;

  constructor(sab: SharedArrayBuffer) {
    this.idx = new Int32Array(sab, 0, 2);
    this.data = new Uint32Array(sab, HEADER_BYTES);
    this.cap = this.data.length / SLOT_WORDS;
  }

  /** Returns false if the ring is full (the command is dropped). */
  push(words: ArrayLike<number>): boolean {
    const w = Atomics.load(this.idx, 0);
    const next = (w + 1) % this.cap;
    if (next === Atomics.load(this.idx, 1)) return false;
    const o = w * SLOT_WORDS;
    for (let i = 0; i < SLOT_WORDS; i++) this.data[o + i] = words[i] ?? 0;
    Atomics.store(this.idx, 0, next); // publish only after the payload is written
    return true;
  }
}

export class RingReader {
  private readonly idx: Int32Array;
  private readonly data: Uint32Array;
  private readonly cap: number;

  constructor(sab: SharedArrayBuffer) {
    this.idx = new Int32Array(sab, 0, 2);
    this.data = new Uint32Array(sab, HEADER_BYTES);
    this.cap = this.data.length / SLOT_WORDS;
  }

  /** Calls `fn` for every pending command; returns how many were read. Allocation-free. */
  drain(fn: (a: number, b: number, c: number, d: number) => void): number {
    let r = Atomics.load(this.idx, 1);
    const w = Atomics.load(this.idx, 0);
    let n = 0;
    while (r !== w) {
      const o = r * SLOT_WORDS;
      fn(this.data[o]!, this.data[o + 1]!, this.data[o + 2]!, this.data[o + 3]!);
      r = (r + 1) % this.cap;
      n++;
    }
    Atomics.store(this.idx, 1, r);
    return n;
  }
}
