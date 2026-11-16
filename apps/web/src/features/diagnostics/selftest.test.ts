// Self-test used by the desktop smoke test in CI and by PO hardware tests.
import { FakeBridge } from '../../test/fakeBridge';
import { runSelfTest } from './selftest';

describe('runSelfTest', () => {
  it('measures command-to-voice latency through the bridge and reports the runtime', async () => {
    const b = new FakeBridge('native');
    const r = await runSelfTest(b, { notes: 5, gapMs: 0 });
    expect(r.kind).toBe('native');
    expect(r.loaded).toBe(true);
    expect(r.samples).toBe(5);
    expect(r.noteToVoiceMedianMs).toBeGreaterThanOrEqual(0);
    expect(r.timeouts).toBe(0);
    expect(b.voices).toBe(0); // every test note was released
    expect(r.recentMaxProcessUs).toBe(80); // #48: both peaks are reported
    expect(r.maxProcessUs).toBe(900);
  });

  it('counts notes that never sounded as timeouts', async () => {
    const b = new FakeBridge();
    b.send = () => {}; // engine ignores commands
    const r = await runSelfTest(b, { notes: 2, gapMs: 0, timeoutMs: 20 });
    expect(r.timeouts).toBe(2);
  });
});
