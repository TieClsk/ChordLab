import type { EngineBridge } from '@chordlab/engine-web';
import { demoProject, LIVE_TRACK } from './demoProject';

export interface SelfTestResult {
  kind: EngineBridge['kind'];
  loaded: boolean;
  device?: string;
  sampleRate: number;
  bufferFrames: number;
  bufferMs: number;
  maxProcessUs?: number;
  recentMaxProcessUs?: number;
  samples: number;
  timeouts: number;
  noteToVoiceMedianMs: number;
  noteToVoiceP95Ms: number;
}

interface Options {
  notes?: number;
  gapMs?: number;
  timeoutMs?: number;
}

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));
const pct = (xs: number[], p: number) => (xs.length ? [...xs].sort((a, b) => a - b)[Math.min(xs.length - 1, Math.floor(xs.length * p))]! : 0);

/** Measures how long a NoteOn takes to become a sounding voice, through the bridge only. */
export async function runSelfTest(bridge: EngineBridge, { notes = 30, gapMs = 150, timeoutMs = 300 }: Options = {}): Promise<SelfTestResult> {
  let loaded = true;
  try {
    await bridge.loadProject(demoProject());
  } catch {
    loaded = false;
  }
  bridge.send({ type: 'stop' });
  await bridge.resume();
  const lat: number[] = [];
  let timeouts = 0;
  for (let i = 0; i < notes; i++) {
    const pitch = 60 + (i % 12);
    const before = (await bridge.status()).activeVoices;
    const t0 = performance.now();
    bridge.send({ type: 'noteOn', track: LIVE_TRACK, pitch, velocity: 100 });
    let sounded = false;
    while (performance.now() - t0 < timeoutMs) {
      if ((await bridge.status()).activeVoices > before) {
        sounded = true;
        break;
      }
      await sleep(1);
    }
    if (sounded) lat.push(performance.now() - t0);
    else timeouts++;
    bridge.send({ type: 'noteOff', track: LIVE_TRACK, pitch });
    if (gapMs) await sleep(gapMs);
  }
  const s = await bridge.status();
  return {
    kind: bridge.kind,
    loaded,
    device: s.device,
    sampleRate: s.sampleRate,
    bufferFrames: s.bufferFrames,
    bufferMs: +((s.bufferFrames / s.sampleRate) * 1000).toFixed(2),
    maxProcessUs: s.maxProcessUs,
    recentMaxProcessUs: s.recentMaxProcessUs,
    samples: lat.length,
    timeouts,
    noteToVoiceMedianMs: +pct(lat, 0.5).toFixed(2),
    noteToVoiceP95Ms: +pct(lat, 0.95).toFixed(2),
  };
}
