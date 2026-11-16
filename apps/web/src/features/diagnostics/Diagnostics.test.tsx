// INF-1 acceptance: the same UI code drives the engine through EngineBridge on any runtime.
import { act, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { I18nextProvider } from 'react-i18next';
import { createI18n } from '../../i18n';
import { FakeBridge } from '../../test/fakeBridge';
import { Diagnostics } from './Diagnostics';

function setup(bridge: FakeBridge | null, error?: string) {
  const user = userEvent.setup();
  render(
    <I18nextProvider i18n={createI18n('en')}>
      <Diagnostics bridge={bridge} error={error} />
    </I18nextProvider>,
  );
  return user;
}

describe('Diagnostics', () => {
  it('names the engine that is running', () => {
    setup(new FakeBridge('native'));
    expect(screen.getByText(/native engine/i)).toBeInTheDocument();
  });

  it('loads the demo project into the engine', async () => {
    const b = new FakeBridge();
    setup(b);
    await waitFor(() => expect(b.projects).toHaveLength(1));
    const synths = b.projects[0]!.tracks.filter((t) => t.instrument.kind === 'synth');
    expect(synths.length).toBeGreaterThan(0);
  });

  it('holding a chord button sounds its notes until it is released', async () => {
    const b = new FakeBridge();
    const user = setup(b);
    await waitFor(() => expect(b.projects).toHaveLength(1));
    const c = screen.getByRole('button', { name: 'C' });
    await user.pointer({ keys: '[MouseLeft>]', target: c });
    expect(b.sent.filter((x) => x.type === 'noteOn').map((x) => (x as { pitch: number }).pitch)).toEqual([60, 64, 67]);
    expect(b.resumed).toBeGreaterThan(0); // browsers need a user gesture to start audio
    await user.pointer({ keys: '[/MouseLeft]', target: c });
    expect(b.sent.filter((x) => x.type === 'noteOff')).toHaveLength(3);
  });

  // #42: OS key auto-repeat and periodic status re-renders must not stack chords
  it('keyboard chords ignore auto-repeat and survive re-renders', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const b = new FakeBridge();
    setup(b);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(300);
    });
    const key = (type: 'keydown' | 'keyup', repeat = false) =>
      act(() => {
        window.dispatchEvent(new KeyboardEvent(type, { key: '1', repeat }));
      });
    key('keydown');
    for (let i = 0; i < 10; i++) {
      key('keydown', true); // OS auto-repeat
      await act(async () => {
        await vi.advanceTimersByTimeAsync(300); // status poll re-renders the page
      });
    }
    key('keydown'); // some platforms repeat without the flag
    expect(b.sent.filter((x) => x.type === 'noteOn')).toHaveLength(3);
    key('keyup');
    expect(b.sent.filter((x) => x.type === 'noteOff')).toHaveLength(3);
    expect(b.voices).toBe(0);
    vi.useRealTimers();
  });

  it('releases held keyboard chords when the window loses focus', async () => {
    const b = new FakeBridge();
    setup(b);
    act(() => {
      window.dispatchEvent(new KeyboardEvent('keydown', { key: '2' }));
    });
    expect(b.voices).toBe(3);
    act(() => {
      window.dispatchEvent(new Event('blur'));
    });
    expect(b.voices).toBe(0);
  });

  it('plays the demo from the start and stops it', async () => {
    const b = new FakeBridge();
    const user = setup(b);
    await user.click(screen.getByRole('button', { name: /play demo/i }));
    expect(b.sent.slice(-2)).toEqual([{ type: 'seek', frame: 0 }, { type: 'play' }]);
    await user.click(screen.getByRole('button', { name: /stop/i }));
    expect(b.sent.at(-1)).toEqual({ type: 'stop' });
  });

  it('shows live engine status', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const b = new FakeBridge();
    setup(b);
    b.voices = 3;
    await act(async () => {
      await vi.advanceTimersByTimeAsync(500);
    });
    expect(screen.getByTestId('active-voices')).toHaveTextContent('3');
    vi.useRealTimers();
  });

  it('explains when the engine could not start', () => {
    setup(null, 'AudioWorklet is required');
    expect(screen.getByRole('alert')).toHaveTextContent('AudioWorklet is required');
  });
});
