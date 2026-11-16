// TypeScript mirror of `engine_core::Command::encode` (ADR-0003).
// Kept in lock-step by the contract test (test/commands.test.ts).

export type EngineCommand =
  | { type: 'play' }
  | { type: 'stop' }
  | { type: 'seek'; frame: number }
  | { type: 'noteOn'; track: number; pitch: number; velocity: number }
  | { type: 'noteOff'; track: number; pitch: number }
  | { type: 'setTrackGain'; track: number; gain: number }
  | { type: 'setTrackPan'; track: number; pan: number }
  | { type: 'setMute'; track: number; mute: boolean }
  | { type: 'setLoop'; start: number; end: number };

/** Four u32 words: [opcode, a, b, c]. */
export type CommandWords = [number, number, number, number];

const OPCODE = { play: 1, stop: 2, seek: 3, noteOn: 4, noteOff: 5, setTrackGain: 6, setTrackPan: 7, setMute: 8, setLoop: 9 } as const;

const f32 = new Float32Array(1);
const u32 = new Uint32Array(f32.buffer);
function f32Bits(x: number): number {
  f32[0] = x;
  return u32[0]!;
}

function int(name: string, v: number, max: number): number {
  if (!Number.isInteger(v) || v < 0 || v > max) throw new RangeError(`${name} must be an integer in 0..=${max}, got ${v}`);
  return v;
}
const u7 = (n: string, v: number) => int(n, v, 127);
const u16 = (n: string, v: number) => int(n, v, 0xffff);
const u32Int = (n: string, v: number) => int(n, v, 0xffff_ffff);

export function encodeCommand(c: EngineCommand): CommandWords {
  const op = OPCODE[c.type];
  switch (c.type) {
    case 'play':
    case 'stop':
      return [op, 0, 0, 0];
    case 'seek': {
      const f = int('frame', c.frame, Number.MAX_SAFE_INTEGER);
      return [op, f % 2 ** 32, Math.floor(f / 2 ** 32), 0];
    }
    case 'noteOn':
      return [op, u16('track', c.track), u7('pitch', c.pitch), u7('velocity', c.velocity)];
    case 'noteOff':
      return [op, u16('track', c.track), u7('pitch', c.pitch), 0];
    case 'setTrackGain':
      return [op, u16('track', c.track), f32Bits(c.gain), 0];
    case 'setTrackPan':
      return [op, u16('track', c.track), f32Bits(c.pan), 0];
    case 'setMute':
      return [op, u16('track', c.track), c.mute ? 1 : 0, 0];
    case 'setLoop':
      return [op, u32Int('start', c.start), u32Int('end', c.end), 0];
  }
}
