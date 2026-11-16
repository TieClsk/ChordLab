import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { EngineBridge, EngineStatus } from '@chordlab/engine-web';
import { demoProject, LIVE_TRACK } from './demoProject';
import { runSelfTest, type SelfTestResult } from './selftest';

const CHORDS: { name: string; notes: number[]; key: string }[] = [
  { name: 'C', notes: [60, 64, 67], key: '1' },
  { name: 'G', notes: [55, 59, 62], key: '2' },
  { name: 'Am', notes: [57, 60, 64], key: '3' },
  { name: 'F', notes: [53, 57, 60], key: '4' },
];

interface Props {
  bridge: EngineBridge | null;
  error?: string;
}

/** Engine check page: proves the UI drives the engine through EngineBridge on any runtime. */
export function Diagnostics({ bridge, error }: Props) {
  const { t } = useTranslation();
  const [status, setStatus] = useState<EngineStatus | null>(null);
  const [result, setResult] = useState<SelfTestResult | null>(null);
  const [running, setRunning] = useState(false);

  useEffect(() => {
    if (!bridge) return;
    void bridge.loadProject(demoProject());
    const timer = setInterval(() => {
      bridge.status().then(setStatus, () => undefined);
    }, 250);
    return () => clearInterval(timer);
  }, [bridge]);

  const press = (notes: number[]) => {
    if (!bridge) return;
    void bridge.resume();
    for (const pitch of notes) bridge.send({ type: 'noteOn', track: LIVE_TRACK, pitch, velocity: 100 });
  };
  const release = (notes: number[]) => {
    for (const pitch of notes) bridge?.send({ type: 'noteOff', track: LIVE_TRACK, pitch });
  };

  useEffect(() => {
    const held = new Set<string>();
    const down = (e: KeyboardEvent) => {
      const c = CHORDS.find((x) => x.key === e.key);
      if (c && !held.has(c.key)) {
        held.add(c.key);
        press(c.notes);
      }
    };
    const up = (e: KeyboardEvent) => {
      const c = CHORDS.find((x) => x.key === e.key);
      if (c) {
        held.delete(c.key);
        release(c.notes);
      }
    };
    window.addEventListener('keydown', down);
    window.addEventListener('keyup', up);
    return () => {
      window.removeEventListener('keydown', down);
      window.removeEventListener('keyup', up);
    };
  });

  if (!bridge) {
    return (
      <section>
        <h2>{t('diagnostics.title')}</h2>
        {error ? <p role="alert">{t('diagnostics.error', { error })}</p> : <p role="status">{t('diagnostics.starting')}</p>}
      </section>
    );
  }

  const playDemo = () => {
    void bridge.resume();
    bridge.send({ type: 'seek', frame: 0 });
    bridge.send({ type: 'play' });
  };
  const selfTest = async () => {
    setRunning(true);
    try {
      setResult(await runSelfTest(bridge));
    } finally {
      setRunning(false);
    }
  };

  return (
    <section className="diagnostics">
      <h2>{t('diagnostics.title')}</h2>
      <p className="muted">{t(bridge.kind === 'native' ? 'diagnostics.runtimeNative' : 'diagnostics.runtimeWorklet')}</p>

      <div className="card">
        <h3>{t('diagnostics.listen')}</h3>
        <button type="button" onClick={playDemo}>
          {t('diagnostics.playDemo')}
        </button>
        <button type="button" onClick={() => bridge.send({ type: 'stop' })}>
          {t('diagnostics.stop')}
        </button>
        <p className="muted">{t('diagnostics.holdChord')}</p>
        <div className="chords">
          {CHORDS.map((c) => (
            <button
              key={c.name}
              type="button"
              className="chord"
              onPointerDown={() => press(c.notes)}
              onPointerUp={() => release(c.notes)}
              onPointerLeave={(e) => e.buttons && release(c.notes)}
            >
              {c.name}
            </button>
          ))}
        </div>
      </div>

      <div className="card">
        <h3>{t('diagnostics.status')}</h3>
        <dl className="status">
          <dt>{t('diagnostics.device')}</dt>
          <dd>{status?.device ?? '—'}</dd>
          <dt>{t('diagnostics.sampleRate')}</dt>
          <dd>{status ? `${status.sampleRate} Hz` : '—'}</dd>
          <dt>{t('diagnostics.buffer')}</dt>
          <dd>{status ? `${status.bufferFrames} (${((status.bufferFrames / status.sampleRate) * 1000).toFixed(1)} ms)` : '—'}</dd>
          <dt>{t('diagnostics.voices')}</dt>
          <dd data-testid="active-voices">{status?.activeVoices ?? 0}</dd>
        </dl>
      </div>

      <div className="card">
        <h3>{t('diagnostics.selfTest')}</h3>
        <button type="button" onClick={() => void selfTest()} disabled={running}>
          {running ? t('diagnostics.selfTestRunning') : t('diagnostics.selfTestRun')}
        </button>
        {result && (
          <>
            <pre>{JSON.stringify(result, null, 2)}</pre>
            <button type="button" onClick={() => void navigator.clipboard?.writeText(JSON.stringify(result, null, 2))}>
              {t('diagnostics.copy')}
            </button>
          </>
        )}
      </div>
    </section>
  );
}
