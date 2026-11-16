// Contract test: TS encoding must equal Rust `Command::encode` (fixture owned by
// crates/engine-core/tests/contract.rs).
import fixture from './fixtures/commands.json';
import { encodeCommand, type EngineCommand } from '../src/commands';

describe('encodeCommand', () => {
  it.each(fixture.map((c) => [c.command.type, c] as const))('%s matches the Rust encoding', (_name, c) => {
    expect(Array.from(encodeCommand(c.command as EngineCommand))).toEqual(c.words);
  });

  it('covers every command type the engine knows', () => {
    const types = new Set(fixture.map((c) => c.command.type));
    expect([...types].sort()).toEqual(['noteOff', 'noteOn', 'play', 'seek', 'setLoop', 'setMute', 'setTrackGain', 'setTrackPan', 'stop']);
  });

  it('rejects out-of-range values instead of silently wrapping', () => {
    expect(() => encodeCommand({ type: 'noteOn', track: 0, pitch: 128, velocity: 100 })).toThrow(RangeError);
    expect(() => encodeCommand({ type: 'noteOn', track: -1, pitch: 60, velocity: 100 })).toThrow(RangeError);
    expect(() => encodeCommand({ type: 'seek', frame: -5 })).toThrow(RangeError);
  });
});
