import type { EngineBridge, EngineCommand, EngineStatus, Project } from '@chordlab/engine-web';

/** In-memory EngineBridge: records everything and simulates voices for note on/off. */
export class FakeBridge implements EngineBridge {
  sent: EngineCommand[] = [];
  projects: Project[] = [];
  voices = 0;
  resumed = 0;
  constructor(public readonly kind: 'native' | 'worklet' = 'worklet') {}

  send(cmd: EngineCommand): void {
    this.sent.push(cmd);
    if (cmd.type === 'noteOn') this.voices++;
    if (cmd.type === 'noteOff') this.voices = Math.max(0, this.voices - 1);
    if (cmd.type === 'stop') this.voices = 0;
  }
  async loadProject(p: Project): Promise<void> {
    this.projects.push(p);
  }
  async status(): Promise<EngineStatus> {
    return { kind: this.kind, sampleRate: 48000, positionFrames: 0, activeVoices: this.voices, bufferFrames: 128, device: 'Fake device', maxProcessUs: 900, recentMaxProcessUs: 80 };
  }
  async resume(): Promise<void> {
    this.resumed++;
  }
  async dispose(): Promise<void> {}
}
