// The UI never chooses an implementation itself (INF-1 acceptance).
import { detectRuntime } from '../src/index';

describe('detectRuntime', () => {
  it('uses the native engine inside the Tauri desktop app', () => {
    expect(detectRuntime({ __TAURI_INTERNALS__: {} })).toBe('tauri');
  });

  it('uses the AudioWorklet engine in a cross-origin isolated browser', () => {
    expect(detectRuntime({ crossOriginIsolated: true, AudioWorkletNode: class {} })).toBe('worklet');
  });

  it('still uses the worklet without isolation (postMessage fallback)', () => {
    expect(detectRuntime({ crossOriginIsolated: false, AudioWorkletNode: class {} })).toBe('worklet');
  });

  it('reports when no audio engine can run', () => {
    expect(detectRuntime({})).toBe('unsupported');
  });
});
