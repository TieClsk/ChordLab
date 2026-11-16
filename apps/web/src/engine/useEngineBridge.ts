import { useEffect, useState } from 'react';
import { createEngineBridge, type EngineBridge } from '@chordlab/engine-web';

/** Path of the WASM engine next to index.html (copied by `pnpm build:wasm`). */
export const WASM_URL = './engine_wasm.wasm';

/** Creates the engine bridge once, when `enabled` first becomes true. */
export function useEngineBridge(enabled: boolean): { bridge: EngineBridge | null; error?: string } {
  const [state, setState] = useState<{ bridge: EngineBridge | null; error?: string }>({ bridge: null });
  useEffect(() => {
    if (!enabled || state.bridge || state.error) return;
    let cancelled = false;
    createEngineBridge({ wasmUrl: WASM_URL }).then(
      (bridge) => !cancelled && setState({ bridge }),
      (e: unknown) => !cancelled && setState({ bridge: null, error: e instanceof Error ? e.message : String(e) }),
    );
    return () => {
      cancelled = true;
    };
  }, [enabled, state.bridge, state.error]);
  return state;
}
