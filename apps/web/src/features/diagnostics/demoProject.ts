import type { Note, Project } from '@chordlab/engine-web';

/** Track 0 is a pad synth so live chord buttons can play on it. */
export const LIVE_TRACK = 0;

/** I–V–vi–IV in C: pad, bass and drums, 4 bars at 100 BPM. */
export function demoProject(): Project {
  const chords = [
    [60, 64, 67],
    [59, 62, 67],
    [57, 60, 64],
    [57, 60, 65],
  ];
  const roots = [36, 43, 45, 41];
  const pad: Note[] = [];
  const bass: Note[] = [];
  const drums: Note[] = [];
  chords.forEach((chord, bar) => {
    const s = bar * 4;
    chord.forEach((pitch, i) => pad.push({ start: s + i * 0.02, length: 3.8, pitch, velocity: 80 + i * 6 }));
    for (let e = 0; e < 8; e++) bass.push({ start: s + e * 0.5, length: 0.42, pitch: roots[bar]!, velocity: e % 2 ? 72 : 100 });
    for (let b = 0; b < 4; b++) {
      drums.push({ start: s + b, length: 0.1, pitch: b % 2 ? 38 : 36, velocity: 110 });
      drums.push({ start: s + b, length: 0.1, pitch: 42, velocity: 64 });
      drums.push({ start: s + b + 0.5, length: 0.1, pitch: 42, velocity: 48 });
    }
  });
  const synth = (cutoff_hz: number, attack_s: number, release_s: number, wave: 'saw' | 'square') => ({
    kind: 'synth' as const,
    wave,
    cutoff_hz,
    resonance: 0.25,
    attack_s,
    decay_s: 0.3,
    sustain: 0.7,
    release_s,
  });
  return {
    tempo_bpm: 100,
    master_gain: 0.8,
    tracks: [
      { name: 'Pad', instrument: synth(2000, 0.06, 0.5, 'saw'), gain: 0.5, pan: -0.15, mute: false, delay_send: 0.2, clips: [{ start_beat: 0, notes: pad }] },
      { name: 'Bass', instrument: synth(650, 0.005, 0.08, 'square'), gain: 0.55, pan: 0, mute: false, delay_send: 0, clips: [{ start_beat: 0, notes: bass }] },
      { name: 'Drums', instrument: { kind: 'drums' }, gain: 0.7, pan: 0, mute: false, delay_send: 0.05, clips: [{ start_beat: 0, notes: drums }] },
    ],
  };
}
