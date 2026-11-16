// INF-2 acceptance: language follows the user's choice, then the system language.
import { detectLanguage, STORAGE_KEY } from './detect';

describe('detectLanguage', () => {
  it('prefers a saved choice over the system language', () => {
    localStorage.setItem(STORAGE_KEY, 'en');
    expect(detectLanguage(['zh-CN'])).toBe('en');
  });

  it('ignores an invalid saved value', () => {
    localStorage.setItem(STORAGE_KEY, 'klingon');
    expect(detectLanguage(['en-US'])).toBe('en');
  });

  it('maps any Chinese variant to zh-CN', () => {
    expect(detectLanguage(['zh-TW'])).toBe('zh-CN');
    expect(detectLanguage(['zh'])).toBe('zh-CN');
  });

  it('uses the first supported system language', () => {
    expect(detectLanguage(['fr-FR', 'zh-CN', 'en'])).toBe('zh-CN');
  });

  it('falls back to English for unsupported languages', () => {
    expect(detectLanguage(['de-DE'])).toBe('en');
    expect(detectLanguage([])).toBe('en');
  });
});
