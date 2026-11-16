// INF-2 acceptance: switching language takes effect immediately and is remembered.
import { render, screen } from '@testing-library/react';
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

  it('marks not-yet-built sections as coming soon', () => {
    renderApp('en');
    const arrange = screen.getByRole('button', { name: /Arrange/ });
    expect(arrange).toBeDisabled();
    expect(arrange).toHaveTextContent(/coming soon/i);
  });
});
