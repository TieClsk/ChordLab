import type { EngineCommand } from './commands';
import type { Project } from './types/Project';

export interface EngineStatus {
  kind: 'native' | 'worklet';
  /** Output device name, when the platform exposes it. */
  device?: string;
  sampleRate: number;
  positionFrames: number;
  activeVoices: number;
  /** Frames rendered per audio callback / render quantum. */
  bufferFrames: number;
  /** Slowest audio callback of the whole session, in microseconds (native only). */
  maxProcessUs?: number;
  /** Slowest audio callback in about the last second (native only, #48). */
  recentMaxProcessUs?: number;
}

/**
 * The only way UI code talks to the audio engine (ADR-0003).
 * Implementations: TauriBridge (desktop, native engine) and WorkletBridge (browser, WASM).
 */
export interface EngineBridge {
  readonly kind: 'native' | 'worklet';
  /** Fire-and-forget real-time command; applied at the next audio block. */
  send(cmd: EngineCommand): void;
  /** Replace the whole project (v1 editing strategy, ADR-0004). */
  loadProject(project: Project): Promise<void>;
  status(): Promise<EngineStatus>;
  /** Browsers only start audio after a user gesture; call this from a click handler. */
  resume(): Promise<void>;
  dispose(): Promise<void>;
}
