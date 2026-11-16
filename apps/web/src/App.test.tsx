// INF-2 acceptance: switching language takes effect immediately and is remembered.
import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { I18nextProvider } from 'react-i18next';
import App from './App';
import { createI18n } from './i18n';
import { STORAGE_KEY } from './i18n/detect';

function renderApp(lang?: 'zh-CN' | 'en') {
  const i18n = createI18n(lang);
  return render(
    <I18nextProvider i18n={i18n}>
      <App />
    </I18nextProvider>,
  );
}

describe('App shell', () => {
  it('shows the product name and Chinese UI when the language is zh-CN', () => {
    renderApp('zh-CN');
    expect(screen.getByRole('heading', { level: 1, name: 'ChordLab' })).toBeInTheDocument();
    expect(screen.getByRole('navigation')).toHaveTextContent('钢琴');
    expect(screen.getByRole('navigation')).toHaveTextContent('教程');
    expect(screen.getByRole('navigation')).toHaveTextContent('分析');
  });

  it('switches to English immediately and remembers the choice', async () => {
    renderApp('zh-CN');
    await userEvent.click(screen.getByRole('button', { name: 'English' }));
    expect(screen.getByRole('navigation')).toHaveTextContent('Piano');
    expect(screen.queryByText('钢琴')).not.toBeInTheDocument();
    expect(localStorage.getItem(STORAGE_KEY)).toBe('en');
    expect(document.documentElement.lang).toBe('en');
  });

  it('restores the remembered language on the next start', () => {
    localStorage.setItem(STORAGE_KEY, 'en');
    renderApp();
    expect(screen.getByRole('navigation')).toHaveTextContent('Lessons');
  });

  // #44: every unfinished module is marked the same way and cannot be opened
  it.each(['Piano', 'Lessons', 'Analysis', 'Arrange'])('marks the unfinished %s module as coming soon', (name) => {
    renderApp('en');
    const item = screen.getByRole('button', { name: new RegExp(name) });
    expect(item).toBeDisabled();
    expect(item).toHaveTextContent(/coming soon/i);
  });

  it('keeps finished modules clickable', () => {
    renderApp('en');
    expect(screen.getByRole('button', { name: 'Engine check' })).toBeEnabled();
  });

  it('home page shows a roadmap with each module, what it does and its planned sprint', () => {
    renderApp('en');
    const roadmap = screen.getByRole('table', { name: /roadmap/i });
    const rows = within(roadmap).getAllByRole('row').slice(1);
    expect(rows.map((r) => within(r).getAllByRole('cell')[0]!.textContent)).toEqual(['Piano', 'Lessons', 'Analysis', 'Arrange']);
    expect(within(rows[0]!).getByText(/S1/)).toBeInTheDocument();
    expect(within(rows[0]!).getByText(/click a chord/i)).toBeInTheDocument();
  });

  it('roadmap is translated too', () => {
    renderApp('zh-CN');
    const roadmap = screen.getByRole('table', { name: '路线图' });
    expect(within(roadmap).getByText('钢琴')).toBeInTheDocument();
  });

  it('can always get back to the home page', async () => {
    renderApp('en');
    await userEvent.click(screen.getByRole('button', { name: 'Engine check' }));
    expect(screen.queryByRole('table', { name: /roadmap/i })).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Home' }));
    expect(screen.getByRole('table', { name: /roadmap/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Home' })).toHaveAttribute('aria-current', 'page');
  });

  it('never shows an empty page: every clickable module renders real content', async () => {
    renderApp('en');
    for (const b of within(screen.getByRole('navigation')).getAllByRole('button')) {
      if ((b as HTMLButtonElement).disabled) continue;
      await userEvent.click(b);
      const main = screen.getByRole('main');
      // more than a heading: controls, data, or an explanation of the engine state
      expect(main.querySelectorAll('button, table, dl, pre, ul, ol, [role=alert], [role=status]').length, b.textContent ?? '').toBeGreaterThan(0);
    }
  });
});
