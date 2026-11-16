// AudioWorkletProcessor hosting the wasm32 ChordLab engine (bundled via `?worker&url`).
// Commands arrive through a SharedArrayBuffer ring when the page is cross-origin
// isolated, otherwise through port messages (fallback).
// NOTE: AudioWorkletGlobalScope has no TextEncoder/TextDecoder/fetch (Spike 001).
import { RingReader } from './ring';

declare const sampleRate: number;
declare function registerProcessor(name: string, ctor: unknown): void;
declare class AudioWorkletProcessor {
  readonly port: MessagePort;
}

interface EngineExports {
  memory: WebAssembly.Memory;
  cl_init(sr: number): void;
  cl_alloc(len: number): number;
  cl_free(ptr: number, len: number): void;
  cl_load_project(ptr: number, len: number): number;
  cl_add_sample(id: number, ptr: number, len: number, sr: number): void;
  cl_add_sample_i16(id: number, ptr: number, len: number, channels: number, sr: number): number;
  cl_command(a: number, b: number, c: number, d: number): number;
  cl_process(frames: number): void;
  cl_out_l(): number;
  cl_out_r(): number;
  cl_position(): number;
  cl_active_voices(): number;
}

type Msg =
  | { type: 'project'; bytes: Uint8Array }
  | { type: 'sample'; id: number; data: Float32Array; sampleRate: number }
  | { type: 'sample-i16'; id: number; data: Int16Array; channels: 1 | 2; sampleRate: number }
  | { type: 'command'; w: [number, number, number, number] };

interface Options {
  wasm: ArrayBuffer;
  ring?: SharedArrayBuffer;
  status?: SharedArrayBuffer;
}

const STATUS_EVERY_BLOCKS = 16;

class ChordLabProcessor extends AudioWorkletProcessor {
  private readonly x: EngineExports;
  private readonly ring: RingReader | null;
  private readonly status: Float64Array | null;
  private blocks = 0;
  private readonly apply = (a: number, b: number, c: number, d: number) => {
    this.x.cl_command(a, b, c, d);
  };

  constructor(options: { processorOptions: Options }) {
    super();
    const o = options.processorOptions;
    const mod = new WebAssembly.Module(o.wasm); // synchronous compile is allowed in worklets
    this.x = new WebAssembly.Instance(mod, {}).exports as unknown as EngineExports;
    this.x.cl_init(sampleRate);
    this.ring = o.ring ? new RingReader(o.ring) : null;
    this.status = o.status ? new Float64Array(o.status) : null;
    this.port.onmessage = (e: MessageEvent<Msg>) => this.onMessage(e.data);
    this.port.postMessage({ type: 'ready' });
  }

  private copyIn(bytes: Uint8Array): [number, number] {
    const p = this.x.cl_alloc(bytes.length);
    new Uint8Array(this.x.memory.buffer, p, bytes.length).set(bytes);
    return [p, bytes.length];
  }

  private onMessage(m: Msg) {
    const x = this.x;
    if (m.type === 'project') {
      const [p, n] = this.copyIn(m.bytes);
      const rc = x.cl_load_project(p, n);
      x.cl_free(p, n);
      this.port.postMessage({ type: 'project-loaded', ok: rc === 0 });
    } else if (m.type === 'sample') {
      const bytes = new Uint8Array(m.data.buffer, m.data.byteOffset, m.data.byteLength);
      const [p, n] = this.copyIn(bytes);
      x.cl_add_sample(m.id, p, m.data.length, m.sampleRate);
      x.cl_free(p, n);
    } else if (m.type === 'sample-i16') {
      const bytes = new Uint8Array(m.data.buffer, m.data.byteOffset, m.data.byteLength);
      const [p, n] = this.copyIn(bytes);
      const rc = x.cl_add_sample_i16(m.id, p, m.data.length, m.channels, m.sampleRate);
      x.cl_free(p, n);
      this.port.postMessage({ type: 'sample-added', id: m.id, ok: rc === 0 });
    } else if (m.type === 'command') {
      x.cl_command(m.w[0], m.w[1], m.w[2], m.w[3]);
    }
  }

  process(_inputs: Float32Array[][], outputs: Float32Array[][]): boolean {
    const out = outputs[0]!;
    const n = out[0]!.length;
    this.ring?.drain(this.apply);
    const x = this.x;
    x.cl_process(n);
    const mem = x.memory.buffer;
    out[0]!.set(new Float32Array(mem, x.cl_out_l(), n));
    out[1]?.set(new Float32Array(mem, x.cl_out_r(), n));
    this.blocks++;
    if (this.status) {
      this.status[0] = x.cl_position();
      this.status[1] = x.cl_active_voices();
      this.status[2] = n;
      this.status[3] = this.blocks;
    } else if (this.blocks % STATUS_EVERY_BLOCKS === 0) {
      this.port.postMessage({ type: 'status', position: x.cl_position(), voices: x.cl_active_voices(), frames: n });
    }
    return true;
  }
}

registerProcessor('chordlab-engine', ChordLabProcessor);
