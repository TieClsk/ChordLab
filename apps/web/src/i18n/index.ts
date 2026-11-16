import i18next, { type i18n as I18n } from 'i18next';
import { initReactI18next } from 'react-i18next';
import en from './locales/en.json';
import zh from './locales/zh-CN.json';
import { detectLanguage, saveLanguage, type Lang } from './detect';

/** Creates an isolated i18n instance (one per app, or per test). */
export function createI18n(lang: Lang = detectLanguage()): I18n {
  const instance = i18next.createInstance();
  void instance.use(initReactI18next).init({
    lng: lang,
    fallbackLng: 'en',
    resources: { en: { translation: en }, 'zh-CN': { translation: zh } },
    interpolation: { escapeValue: false }, // React already escapes
    initAsync: false,
  });
  document.documentElement.lang = lang;
  instance.on('languageChanged', (l) => {
    document.documentElement.lang = l;
    saveLanguage(l as Lang);
  });
  return instance;
}

export type { Lang };
export { SUPPORTED } from './detect';
