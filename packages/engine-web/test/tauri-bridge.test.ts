// Desktop implementation: commands go through Tauri `invoke` to the native engine.
import { TauriBridge, type Invoke } from '../src/tauri-bridge';
import type { Project } from '../src/types/Project';

function fakeInvoke(responses: Record<string, unknown> = {}) {
  const calls: { cmd: string; args?: Record<string, unknown> }[] = [];
  const invoke: Invoke = async (cmd, args) => {
    calls.push({ cmd, args });
    return responses[cmd] as never;
  };
  return { invoke, calls };
}

const project: Project = { tempo_bpm: 100, master_gain: 0.8, tracks: [] };

describe('TauriBridge', () => {
  it('sends encoded commands to engine_cmd', async () => {
    const { invoke, calls } = fakeInvoke({ engine_cmd: true });
    const b = new TauriBridge(invoke);
    b.send({ type: 'noteOn', track: 2, pitch: 64, velocity: 90 });
    b.send({ type: 'stop' });
    await b.flush();
    expect(calls).toEqual([
      { cmd: 'engine_cmd', args: { w: [4, 2, 64, 90] } },
      { cmd: 'engine_cmd', args: { w: [2, 0, 0, 0] } },
    ]);
  });

  it('loads a project as JSON through load_project', async () => {
    const { invoke, calls } = fakeInvoke({ load_project: true });
    await new TauriBridge(invoke).loadProject(project);
    expect(calls).toEqual([{ cmd: 'load_project', args: { json: JSON.stringify(project) } }]);
  });

  it('reports a failed project load', async () => {
    const { invoke } = fakeInvoke({ load_project: false });
    await expect(new TauriBridge(invoke).loadProject(project)).rejects.toThrow(/load/i);
  });

  it('maps native status to the common EngineStatus shape', async () => {
    const { invoke } = fakeInvoke({
      engine_status: { device: 'Speakers', sampleRate: 48000, position: 96000, voices: 3, callbacks: 10, maxCallbackUs: 120, framesPerCallback: 480 },
    });
    expect(await new TauriBridge(invoke).status()).toEqual({
      kind: 'native',
      device: 'Speakers',
      sampleRate: 48000,
      positionFrames: 96000,
      activeVoices: 3,
      bufferFrames: 480,
      maxProcessUs: 120,
    });
  });

  it('turns a missing audio device into a clear error', async () => {
    const { invoke } = fakeInvoke({ engine_status: { error: 'no audio device' } });
    await expect(new TauriBridge(invoke).status()).rejects.toThrow('no audio device');
  });
});
