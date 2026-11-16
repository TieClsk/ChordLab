import { createEngineBridge } from '@chordlab/engine-web';
import { runSelfTest } from '../features/diagnostics/selftest';
import { WASM_URL } from './useEngineBridge';

type Invoke = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;

/**
 * Desktop CI smoke test: when the app is started with CHORDLAB_SELFTEST set, run the
 * self-test through the real bridge and report the result to the native side.
 */
export async function selftestOnStartup(): Promise<void> {
  const internals = (globalThis as unknown as { __TAURI_INTERNALS__?: { invoke: Invoke } }).__TAURI_INTERNALS__;
  if (!internals) return;
  if (!(await internals.invoke<boolean>('selftest_requested'))) return;
  try {
    const bridge = await createEngineBridge({ wasmUrl: WASM_URL });
    const result = await runSelfTest(bridge, { notes: 10, gapMs: 50 });
    await internals.invoke('report', { json: JSON.stringify(result) });
  } catch (e) {
    await internals.invoke('report', { json: JSON.stringify({ error: String(e) }) });
  }
}
