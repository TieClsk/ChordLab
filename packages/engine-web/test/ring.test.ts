// The UI thread writes 16-byte commands, the AudioWorklet reads them (ADR-0003).
import { createRingBuffer, RingReader, RingWriter } from '../src/ring';

const drainAll = (r: RingReader) => {
  const out: number[][] = [];
  r.drain((a, b, c, d) => out.push([a, b, c, d]));
  return out;
};

describe('SharedArrayBuffer command ring', () => {
  it('delivers commands in order', () => {
    const sab = createRingBuffer(8);
    const w = new RingWriter(sab);
    const r = new RingReader(sab);
    w.push([4, 0, 60, 100]);
    w.push([5, 0, 60, 0]);
    expect(drainAll(r)).toEqual([
      [4, 0, 60, 100],
      [5, 0, 60, 0],
    ]);
    expect(drainAll(r)).toEqual([]);
  });

  it('holds capacity - 1 items and reports when full', () => {
    const sab = createRingBuffer(4);
    const w = new RingWriter(sab);
    expect(w.push([1, 0, 0, 0])).toBe(true);
    expect(w.push([2, 0, 0, 0])).toBe(true);
    expect(w.push([3, 0, 0, 0])).toBe(true);
    expect(w.push([4, 0, 0, 0])).toBe(false);
  });

  it('wraps around many times without losing or reordering commands', () => {
    const sab = createRingBuffer(5);
    const w = new RingWriter(sab);
    const r = new RingReader(sab);
    const seen: number[] = [];
    for (let i = 0; i < 1000; i++) {
      expect(w.push([i, i + 1, i + 2, i + 3])).toBe(true);
      if (i % 3 === 2) for (const [a] of drainAll(r)) seen.push(a!);
    }
    for (const [a] of drainAll(r)) seen.push(a!);
    expect(seen).toEqual(Array.from({ length: 1000 }, (_, i) => i));
  });
});
