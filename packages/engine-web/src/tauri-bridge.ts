import type { EngineBridge, EngineStatus } from './bridge';
import { encodeCommand, type EngineCommand } from './commands';
import type { Project } from './types/Project';

export type Invoke = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;

interface NativeStatus {
  error?: string;
  device: string;
  sampleRate: number;
  position: number;
  voices: number;
  maxCallbackUs: number;
  framesPerCallback: number;
}

/** Desktop: the engine runs natively on the cpal audio thread (apps/desktop). */
export class TauriBridge implements EngineBridge {
  readonly kind = 'native' as const;
  /** Commands are chained so they reach the engine in the order they were sent. */
  private queue: Promise<unknown> = Promise.resolve();

  constructor(private readonly invoke: Invoke) {}

  send(cmd: EngineCommand): void {
    const w = encodeCommand(cmd);
    this.queue = this.queue.then(() => this.invoke('engine_cmd', { w })).catch(() => undefined);
  }

  /** Resolves when every command sent so far has been handed to the engine. */
  async flush(): Promise<void> {
    await this.queue;
  }

  async loadProject(project: Project): Promise<void> {
    await this.flush();
    const ok = await this.invoke<boolean>('load_project', { json: JSON.stringify(project) });
    if (!ok) throw new Error('Native engine could not load the project');
  }

  async status(): Promise<EngineStatus> {
    const s = await this.invoke<NativeStatus>('engine_status');
    if (s.error) throw new Error(s.error);
    return {
      kind: 'native',
      device: s.device,
      sampleRate: s.sampleRate,
      positionFrames: s.position,
      activeVoices: s.voices,
      bufferFrames: s.framesPerCallback,
      maxProcessUs: s.maxCallbackUs,
    };
  }

  async resume(): Promise<void> {
    /* native audio is always running */
  }

  async dispose(): Promise<void> {
    await this.flush();
  }
}
