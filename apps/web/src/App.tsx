import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { SUPPORTED, type Lang } from './i18n';
import { Diagnostics } from './features/diagnostics/Diagnostics';
import { useEngineBridge } from './engine/useEngineBridge';
import './App.css';

type Section = 'piano' | 'lessons' | 'analysis' | 'arrange' | 'diagnostics';

/**
 * Modules, whether they can be opened yet and when they are planned
 * (docs/product/backlog.md). A module is only marked ready once it has a real page (#44).
 */
const SECTIONS: { id: Section; ready: boolean; sprint?: string }[] = [
  { id: 'piano', ready: false, sprint: 'S1–S2' },
  { id: 'lessons', ready: false, sprint: 'S3' },
  { id: 'analysis', ready: false, sprint: 'S4–S6' },
  { id: 'arrange', ready: false, sprint: 'S7–S8' },
  { id: 'diagnostics', ready: true },
];
const ROADMAP = SECTIONS.filter((x) => x.sprint);

const LANG_LABEL: Record<Lang, string> = { 'zh-CN': '中文', en: 'English' };

export default function App() {
  const { t, i18n } = useTranslation();
  const [section, setSection] = useState<Section | null>(null);
  // The engine starts only when a page needs it (browsers also need a user gesture).
  const engine = useEngineBridge(section === 'diagnostics');

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
        <button type="button" aria-current={section === null ? 'page' : undefined} onClick={() => setSection(null)}>
          <span>{t('nav.home')}</span>
        </button>
        {SECTIONS.map(({ id, ready }) => (
          <button key={id} type="button" disabled={!ready} aria-current={section === id ? 'page' : undefined} onClick={() => setSection(id)}>
            <span>{t(`nav.${id}`)}</span>
            {!ready && <small>{t('app.comingSoon')}</small>}
          </button>
        ))}
      </nav>

      <main className="content">
        {section === 'diagnostics' ? (
          <Diagnostics bridge={engine.bridge} error={engine.error} />
        ) : (
          <>
            <h2>{t('home.title')}</h2>
            <p>{t('home.body')}</p>
            <table className="roadmap">
              <caption>{t('home.roadmap')}</caption>
              <thead>
                <tr>
                  <th scope="col">{t('home.module')}</th>
                  <th scope="col">{t('home.what')}</th>
                  <th scope="col">{t('home.planned')}</th>
                </tr>
              </thead>
              <tbody>
                {ROADMAP.map(({ id, sprint }) => (
                  <tr key={id}>
                    <td>{t(`nav.${id}`)}</td>
                    <td>{t(`section.${id}`)}</td>
                    <td>{sprint}</td>
                  </tr>
                ))}
              </tbody>
            </table>
            <p className="muted">{t('home.sprint', { sprint: 'S0' })}</p>
          </>
        )}
      </main>
    </div>
  );
}
