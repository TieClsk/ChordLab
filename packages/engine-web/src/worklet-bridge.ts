import type { EngineBridge, EngineStatus } from './bridge';
import { encodeCommand, type EngineCommand } from './commands';
import { createRingBuffer, RingWriter } from './ring';
import type { Project } from './types/Project';
import workletUrl from './worklet.ts?worker&url';

interface CreateOptions {
  wasmUrl: string;
  context?: AudioContext;
}

function waitFor<T extends { type: string }>(port: MessagePort, type: string, timeoutMs = 10_000): Promise<T> {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error(`engine did not answer "${type}" in ${timeoutMs} ms`)), timeoutMs);
    const onMsg = (e: MessageEvent<T>) => {
      if (e.data?.type === type) {
        clearTimeout(timer);
        port.removeEventListener('message', onMsg);
        resolve(e.data);
      }
    };
    port.addEventListener('message', onMsg);
    port.start();
  });
}

/** Browser: the WASM engine runs inside an AudioWorklet (ADR-0001, ADR-0003). */
export class WorkletBridge implements EngineBridge {
  readonly kind = 'worklet' as const;
  private lastStatus = { position: 0, voices: 0, frames: 128 };

  private constructor(
    private readonly ctx: AudioContext,
    private readonly node: AudioWorkletNode,
    private readonly ring: RingWriter | null,
    private readonly statusView: Float64Array | null,
  ) {
    if (!statusView) {
      node.port.addEventListener('message', (e: MessageEvent<{ type: string; position: number; voices: number; frames: number }>) => {
        if (e.data?.type === 'status') this.lastStatus = e.data;
      });
    }
  }

  static async create({ wasmUrl, context }: CreateOptions): Promise<WorkletBridge> {
    const ctx = context ?? new AudioContext({ latencyHint: 'interactive' });
    const wasmResp = await fetch(wasmUrl);
    if (!wasmResp.ok) throw new Error(`could not load ${wasmUrl} (${wasmResp.status})`);
    const wasm = await wasmResp.arrayBuffer();
    await ctx.audioWorklet.addModule(workletUrl);
    // SharedArrayBuffer is only available when the page is cross-origin isolated.
    const shared = typeof SharedArrayBuffer !== 'undefined' && globalThis.crossOriginIsolated === true;
    const ringSab = shared ? createRingBuffer(1024) : undefined;
    const statusSab = shared ? new SharedArrayBuffer(4 * 8) : undefined;
    const node = new AudioWorkletNode(ctx, 'chordlab-engine', {
      numberOfInputs: 0,
      outputChannelCount: [2],
      processorOptions: { wasm, ring: ringSab, status: statusSab },
    });
    await waitFor(node.port, 'ready');
    node.connect(ctx.destination);
    return new WorkletBridge(ctx, node, ringSab ? new RingWriter(ringSab) : null, statusSab ? new Float64Array(statusSab) : null);
  }

  /** True when commands travel through shared memory instead of postMessage. */
  get lockFree(): boolean {
    return this.ring !== null;
  }

  send(cmd: EngineCommand): void {
    const w = encodeCommand(cmd);
    if (!this.ring?.push(w)) this.node.port.postMessage({ type: 'command', w });
  }

  async loadProject(project: Project): Promise<void> {
    // The worklet has no TextEncoder: encode on the main thread (Spike 001).
    const bytes = new TextEncoder().encode(JSON.stringify(project));
    const done = waitFor<{ type: string; ok: boolean }>(this.node.port, 'project-loaded');
    this.node.port.postMessage({ type: 'project', bytes }, [bytes.buffer]);
    if (!(await done).ok) throw new Error('WASM engine could not parse the project');
  }

  async status(): Promise<EngineStatus> {
    const s = this.statusView;
    return {
      kind: 'worklet',
      sampleRate: this.ctx.sampleRate,
      positionFrames: s ? s[0]! : this.lastStatus.position,
      activeVoices: s ? s[1]! : this.lastStatus.voices,
      bufferFrames: s ? s[2]! || 128 : this.lastStatus.frames,
    };
  }

  async resume(): Promise<void> {
    if (this.ctx.state !== 'running') await this.ctx.resume();
  }

  async dispose(): Promise<void> {
    this.node.disconnect();
    await this.ctx.close();
  }
}
