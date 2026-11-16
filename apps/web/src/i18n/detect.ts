export const SUPPORTED = ['zh-CN', 'en'] as const;
export type Lang = (typeof SUPPORTED)[number];
export const STORAGE_KEY = 'chordlab.lang';

function readStored(): Lang | null {
  try {
    const v = localStorage.getItem(STORAGE_KEY);
    return (SUPPORTED as readonly string[]).includes(v ?? '') ? (v as Lang) : null;
  } catch {
    return null; // storage can be unavailable (private mode, sandboxed previews)
  }
}

function fromSystem(tag: string): Lang | null {
  const base = tag.toLowerCase().split('-')[0];
  if (base === 'zh') return 'zh-CN';
  if (base === 'en') return 'en';
  return null;
}

/** Saved choice first, then the first supported system language, then English. */
export function detectLanguage(systemLanguages: readonly string[] = navigator.languages): Lang {
  const stored = readStored();
  if (stored) return stored;
  for (const tag of systemLanguages) {
    const lang = fromSystem(tag);
    if (lang) return lang;
  }
  return 'en';
}

export function saveLanguage(lang: Lang): void {
  try {
    localStorage.setItem(STORAGE_KEY, lang);
  } catch {
    /* not persisted; still applied for this session */
  }
}
