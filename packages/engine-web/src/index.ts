import type { EngineBridge } from './bridge';
import type { Invoke } from './tauri-bridge';

export type { EngineBridge, EngineStatus } from './bridge';
export { encodeCommand, type EngineCommand } from './commands';
export type { Project } from './types/Project';
export type { Track } from './types/Track';
export type { Clip } from './types/Clip';
export type { Note } from './types/Note';
export type { Instrument } from './types/Instrument';
export type { SynthParams } from './types/SynthParams';

export type Runtime = 'tauri' | 'worklet' | 'unsupported';

/** Decides which engine implementation can run here. */
export function detectRuntime(g: Record<string, unknown> = globalThis as unknown as Record<string, unknown>): Runtime {
  if (g.__TAURI_INTERNALS__) return 'tauri';
  if (typeof g.AudioWorkletNode === 'function') return 'worklet';
  return 'unsupported';
}

export interface CreateOptions {
  /** URL of engine_wasm.wasm (browser only). */
  wasmUrl: string;
}

/** Creates the right bridge for this runtime. Implementations are loaded lazily. */
export async function createEngineBridge(opts: CreateOptions): Promise<EngineBridge> {
  const runtime = detectRuntime();
  if (runtime === 'tauri') {
    const { TauriBridge } = await import('./tauri-bridge');
    const internals = (globalThis as unknown as { __TAURI_INTERNALS__: { invoke: Invoke } }).__TAURI_INTERNALS__;
    return new TauriBridge((cmd, args) => internals.invoke(cmd, args));
  }
  if (runtime === 'worklet') {
    const { WorkletBridge } = await import('./worklet-bridge');
    return WorkletBridge.create({ wasmUrl: opts.wasmUrl });
  }
  throw new Error('This browser cannot run the ChordLab audio engine (AudioWorklet is required).');
}
