// INF-2 acceptance: every user-visible string exists in both languages.
import zh from './locales/zh-CN.json';
import en from './locales/en.json';

type Tree = { [k: string]: string | Tree };

function flatten(tree: Tree, prefix = ''): Record<string, string> {
  const out: Record<string, string> = {};
  for (const [k, v] of Object.entries(tree)) {
    const key = prefix ? `${prefix}.${k}` : k;
    if (typeof v === 'string') out[key] = v;
    else Object.assign(out, flatten(v, key));
  }
  return out;
}

const zhFlat = flatten(zh as Tree);
const enFlat = flatten(en as Tree);

describe('locales', () => {
  it('zh-CN and en define exactly the same keys', () => {
    expect(Object.keys(enFlat).sort()).toEqual(Object.keys(zhFlat).sort());
  });

  it('has no empty translations', () => {
    for (const [k, v] of [...Object.entries(zhFlat), ...Object.entries(enFlat)]) {
      expect(v.trim(), k).not.toBe('');
    }
  });

  it('keeps interpolation placeholders identical across languages', () => {
    const vars = (s: string) => (s.match(/\{\{\s*\w+\s*\}\}/g) ?? []).sort();
    for (const k of Object.keys(zhFlat)) {
      expect(vars(enFlat[k] ?? ''), k).toEqual(vars(zhFlat[k] ?? ''));
    }
  });
});
