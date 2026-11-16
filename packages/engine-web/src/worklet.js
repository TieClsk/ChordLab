// AudioWorkletProcessor hosting the wasm32 ChordLab engine.
// processorOptions: { wasm: ArrayBuffer, ring?: SharedArrayBuffer, status?: SharedArrayBuffer }
// - commands arrive through the SAB ring (no postMessage on the hot path)
// - status SAB (Float64Array[4]): [position frames, active voices, max process µs, blocks]
import { RingReader } from './ring.js';

class ChordLabProcessor extends AudioWorkletProcessor {
  constructor(options) {
    super();
    const o = options.processorOptions;
    const mod = new WebAssembly.Module(o.wasm); // sync compile is allowed inside worklets
    this.x = new WebAssembly.Instance(mod, {}).exports;
    this.x.cl_init(sampleRate);
    this.ring = o.ring ? new RingReader(o.ring) : null;
    this.status = o.status ? new Float64Array(o.status) : null;
    this.apply = (a, b, c, d) => this.x.cl_command(a, b, c, d);
    this.port.onmessage = (e) => this.onMessage(e.data);
    this.maxUs = 0;
    this.blocks = 0;
    this.port.postMessage({ type: 'ready', memoryBytes: this.x.memory.buffer.byteLength });
  }

  onMessage(m) {
    const x = this.x;
    if (m.type === 'project') {
      // AudioWorkletGlobalScope has no TextEncoder/TextDecoder: the main thread
      // must send the project JSON already UTF-8 encoded (Uint8Array).
      const bytes = m.bytes;
      const p = x.cl_alloc(bytes.length);
      new Uint8Array(x.memory.buffer, p, bytes.length).set(bytes);
      const rc = x.cl_load_project(p, bytes.length);
      x.cl_free(p, bytes.length);
      this.port.postMessage({ type: 'project-loaded', rc });
    } else if (m.type === 'sample') {
      const p = x.cl_alloc(m.data.length * 4);
      new Float32Array(x.memory.buffer, p, m.data.length).set(m.data);
      x.cl_add_sample(m.id, p, m.data.length, m.sampleRate);
      x.cl_free(p, m.data.length * 4);
    } else if (m.type === 'demo') {
      x.cl_add_test_tone(0);
      x.cl_load_demo(1, m.bars);
      this.port.postMessage({ type: 'project-loaded', rc: 0 });
    } else if (m.type === 'command') {
      x.cl_command(m.a, m.b ?? 0, m.c ?? 0, m.d ?? 0);
    }
  }

  process(_inputs, outputs) {
    const out = outputs[0];
    const n = out[0].length;
    if (this.ring) this.ring.drain(this.apply);
    const x = this.x;
    x.cl_process(n);
    const mem = x.memory.buffer;
    out[0].set(new Float32Array(mem, x.cl_out_l(), n));
    if (out[1]) out[1].set(new Float32Array(mem, x.cl_out_r(), n));
    if (this.status) {
      this.status[0] = x.cl_position();
      this.status[1] = x.cl_active_voices();
      this.status[3] = ++this.blocks;
    }
    return true;
  }
}

registerProcessor('chordlab-engine', ChordLabProcessor);
