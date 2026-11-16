import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { SUPPORTED, type Lang } from './i18n';
import './App.css';

type Section = 'piano' | 'lessons' | 'analysis' | 'arrange';

/** Sections and whether they are available yet (Sprint plan: docs/product/backlog.md). */
const SECTIONS: { id: Section; ready: boolean }[] = [
  { id: 'piano', ready: true },
  { id: 'lessons', ready: true },
  { id: 'analysis', ready: true },
  { id: 'arrange', ready: false },
];

const LANG_LABEL: Record<Lang, string> = { 'zh-CN': '中文', en: 'English' };

export default function App() {
  const { t, i18n } = useTranslation();
  const [section, setSection] = useState<Section | null>(null);

  return (
    <div className="shell">
      <header className="topbar">
        <div className="brand">
          <h1>ChordLab</h1>
          <p className="tagline">{t('app.tagline')}</p>
        </div>
        <div className="lang" role="group" aria-label={t('app.language')}>
          {SUPPORTED.map((l) => (
            <button key={l} type="button" aria-pressed={i18n.language === l} onClick={() => void i18n.changeLanguage(l)}>
              {LANG_LABEL[l]}
            </button>
          ))}
        </div>
      </header>

      <nav className="sidenav">
        {SECTIONS.map(({ id, ready }) => (
          <button key={id} type="button" disabled={!ready} aria-current={section === id ? 'page' : undefined} onClick={() => setSection(id)}>
            <span>{t(`nav.${id}`)}</span>
            {!ready && <small>{t('app.comingSoon')}</small>}
          </button>
        ))}
      </nav>

      <main className="content">
        {section === null ? (
          <>
            <h2>{t('home.title')}</h2>
            <p>{t('home.body')}</p>
            <p className="muted">{t('home.sprint', { sprint: 'S0' })}</p>
          </>
        ) : (
          <>
            <h2>{t(`nav.${section}`)}</h2>
            <p>{t(`section.${section}`)}</p>
          </>
        )}
      </main>
    </div>
  );
}
