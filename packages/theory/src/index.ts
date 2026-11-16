// ChordLab music theory core (#10 E0-1). Wraps tonal and fixes the issues recorded in
// Spike 000: pop-style chord symbols, roman numerals in common notation ("vi", not
// "VIm"), detection ranked for real playing (C/E before Em#5), and octave-aware voicings.
import { Chord, Interval, Key, Note } from 'tonal';

export type Quality =
  | 'major'
  | 'minor'
  | 'diminished'
  | 'augmented'
  | 'sus2'
  | 'sus4'
  | 'major6'
  | 'minor6'
  | 'dominant7'
  | 'major7'
  | 'minor7'
  | 'halfDiminished7'
  | 'diminished7'
  | 'minorMajor7'
  | 'add9'
  | 'dominant9'
  | 'other';

export interface ChordInfo {
  /** Normalised symbol, e.g. "Cmaj7", "Am7", "G7/B". */
  symbol: string;
  root: string;
  /** Lowest note: the root unless it is a slash chord. */
  bass: string;
  quality: Quality;
  /** Pitch classes, bass first. */
  notes: string[];
}

/** tonal type name -> (our quality, pop suffix). */
const TYPES: Record<string, [Quality, string]> = {
  major: ['major', ''],
  minor: ['minor', 'm'],
  diminished: ['diminished', 'dim'],
  augmented: ['augmented', 'aug'],
  'suspended second': ['sus2', 'sus2'],
  'suspended fourth': ['sus4', 'sus4'],
  sixth: ['major6', '6'],
  'minor sixth': ['minor6', 'm6'],
  'dominant seventh': ['dominant7', '7'],
  'major seventh': ['major7', 'maj7'],
  'minor seventh': ['minor7', 'm7'],
  'half-diminished': ['halfDiminished7', 'm7b5'],
  'diminished seventh': ['diminished7', 'dim7'],
  'minor/major seventh': ['minorMajor7', 'm(maj7)'],
  'dominant ninth': ['dominant9', '9'],
};

/** Qualities a pop/rock player meets every day; used to rank detection results. */
const COMMON = new Set<Quality>(['major', 'minor', 'dominant7', 'major7', 'minor7', 'diminished', 'halfDiminished7', 'sus4', 'sus2', 'major6', 'minor6', 'add9']);

function classify(c: ReturnType<typeof Chord.get>): [Quality, string] {
  const known = TYPES[c.type];
  if (known) return known;
  if (c.aliases.includes('add9')) return ['add9', 'add9']; // tonal leaves its type empty
  return ['other', c.aliases[0] ?? ''];
}

/** Parses a chord symbol. Throws on symbols tonal cannot read. */
export function chord(symbol: string): ChordInfo {
  const c = Chord.get(symbol);
  if (c.empty || !c.tonic) throw new Error(`Unknown chord symbol: "${symbol}"`);
  const [quality, suffix] = classify(c);
  const bass = c.bass || c.tonic;
  const slash = bass !== c.tonic ? `/${bass}` : '';
  return { symbol: `${c.tonic}${suffix}${slash}`, root: c.tonic, bass, quality, notes: c.notes };
}

export interface VoicingOptions {
  /** Octave of the root (4 = middle C octave). */
  octave?: number;
  /** 0 = root position, 1 = first inversion, ... */
  inversion?: number;
}

/** MIDI notes to play and highlight, ascending. */
export function chordMidi(symbol: string, { octave = 4, inversion = 0 }: VoicingOptions = {}): number[] {
  const info = chord(symbol);
  const body = Chord.get(info.symbol.split('/')[0]!).notes; // chord without its slash bass
  const midi: number[] = [];
  for (const pc of body) {
    let m = Note.midi(`${pc}${octave}`)!;
    while (midi.length && m <= midi[midi.length - 1]!) m += 12;
    midi.push(m);
  }
  for (let i = 0; i < inversion % midi.length; i++) midi.push(midi.shift()! + 12);
  if (info.bass !== info.root) {
    let b = Note.midi(`${info.bass}${octave}`)!;
    while (b >= midi[0]!) b -= 12;
    midi.unshift(b);
  }
  return midi;
}

export interface DetectOptions {
  /** Key tonic (major), used to prefer diatonic readings. */
  key?: string;
}

/** Possible chord names for some notes, best first. The first note (or lowest MIDI note) is the bass. */
export function detectChord(input: (string | number)[], { key }: DetectOptions = {}): string[] {
  const pcs = (typeof input[0] === 'number' ? [...(input as number[])].sort((a, b) => a - b).map((m) => Note.pitchClass(Note.fromMidi(m))) : (input as string[]).map(Note.pitchClass));
  const unique = pcs.filter((p, i) => pcs.findIndex((q) => Note.chroma(q) === Note.chroma(p)) === i);
  if (unique.length < 2) return [];
  const bass = unique[0]!;
  const diatonic = key ? new Set(diatonicChords(key, { sevenths: false }).concat(diatonicChords(key, { sevenths: true }))) : null;
  const scored = Chord.detect(unique).flatMap((name) => {
    try {
      const c = chord(name);
      let score = 0;
      if (COMMON.has(c.quality)) score += 3;
      if (Note.chroma(c.root) === Note.chroma(bass)) score += 2;
      if (diatonic?.has(c.symbol.split('/')[0]!)) score += 1;
      return [{ symbol: c.symbol, score }];
    } catch {
      return [];
    }
  });
  return scored.sort((a, b) => b.score - a.score).map((s) => s.symbol);
}

const NUMERALS = ['I', 'II', 'III', 'IV', 'V', 'VI', 'VII'];
const MAJOR_SCALE = ['1P', '2M', '3M', '4P', '5P', '6M', '7M'];
const LOWER = new Set<Quality>(['minor', 'diminished', 'halfDiminished7', 'diminished7', 'minor6', 'minor7', 'minorMajor7']);
const ROMAN_SUFFIX: Partial<Record<Quality, string>> = {
  diminished: '°',
  augmented: '+',
  sus2: 'sus2',
  sus4: 'sus4',
  major6: '6',
  minor6: '6',
  dominant7: '7',
  major7: 'maj7',
  minor7: '7',
  halfDiminished7: 'ø7',
  diminished7: '°7',
  minorMajor7: '(maj7)',
  add9: 'add9',
  dominant9: '9',
};

/** Degree (1-7) and accidental of `note` relative to the major scale of `tonic`. */
function degree(tonic: string, note: string): { step: number; accidental: string } {
  const iv = Interval.get(Interval.distance(tonic, note));
  const step = ((iv.num ?? 1) - 1) % 7;
  const semis = ((iv.semitones ?? 0) % 12 + 12) % 12;
  const expected = Interval.semitones(MAJOR_SCALE[step]!) ?? 0;
  const diff = semis - expected;
  const accidental = diff === -1 || diff === 11 ? '♭' : diff === 1 || diff === -11 ? '♯' : diff === -2 ? '𝄫' : diff === 2 ? '𝄪' : '';
  return { step, accidental };
}

/** Roman numeral of a chord in a major key, e.g. ("Am7", "C") -> "vi7". */
export function romanNumeral(symbol: string, key: string): string {
  const c = chord(symbol);
  const { step, accidental } = degree(key, c.root);
  let numeral = NUMERALS[step]!;
  if (LOWER.has(c.quality)) numeral = numeral.toLowerCase();
  const suffix = ROMAN_SUFFIX[c.quality] ?? '';
  let figure = '';
  if (c.bass !== c.root) {
    const iv = Interval.get(Interval.distance(c.root, c.bass));
    figure = `/${iv.num}`;
  }
  return `${accidental}${numeral}${suffix}${figure}`;
}

const ROMAN_RE = /^([♭♯b#]?)(VII|VI|IV|V|III|II|I|vii|vi|iv|v|iii|ii|i)(.*)$/;
const SUFFIX_TO_QUALITY: [string, boolean, Quality][] = [
  // [roman suffix, lower case numeral?, quality]
  ['', false, 'major'],
  ['', true, 'minor'],
  ['°', true, 'diminished'],
  ['+', false, 'augmented'],
  ['7', false, 'dominant7'],
  ['7', true, 'minor7'],
  ['maj7', false, 'major7'],
  ['ø7', true, 'halfDiminished7'],
  ['°7', true, 'diminished7'],
  ['6', false, 'major6'],
  ['6', true, 'minor6'],
  ['sus2', false, 'sus2'],
  ['sus4', false, 'sus4'],
  ['add9', false, 'add9'],
  ['9', false, 'dominant9'],
];
const QUALITY_SUFFIX: Record<Quality, string> = Object.fromEntries(
  Object.values(TYPES).map(([q, s]) => [q, s]).concat([['add9', 'add9'], ['other', '']]),
) as Record<Quality, string>;

/** Chords for roman numerals in a major key, e.g. (["I","V","vi","IV"], "Eb") -> ["Eb","Bb","Cm","Ab"]. */
export function fromRomans(romans: string[], key: string): string[] {
  return romans.map((r) => {
    const m = ROMAN_RE.exec(r.trim());
    if (!m) throw new Error(`Unknown roman numeral: "${r}"`);
    const [, acc, numeral, suffix] = m as unknown as [string, string, string, string];
    const lower = numeral === numeral.toLowerCase();
    const step = NUMERALS.indexOf(numeral.toUpperCase());
    let interval = MAJOR_SCALE[step]!;
    if (acc === '♭' || acc === 'b') interval = Interval.subtract(interval, '1A') ?? interval;
    if (acc === '♯' || acc === '#') interval = Interval.add(interval, '1A') ?? interval;
    const root = Note.pitchClass(Note.transpose(key, interval));
    const entry = SUFFIX_TO_QUALITY.find(([s, l]) => s === suffix && l === lower);
    if (!entry) throw new Error(`Unknown roman numeral suffix: "${r}"`);
    return chord(`${root}${QUALITY_SUFFIX[entry[2]]}`).symbol;
  });
}

export interface DiatonicOptions {
  sevenths?: boolean;
}

/** Diatonic chords of a major key, I to vii°. */
export function diatonicChords(key: string, { sevenths = false }: DiatonicOptions = {}): string[] {
  const k = Key.majorKey(key);
  return (sevenths ? k.chords : k.triads).map((c) => chord(c).symbol);
}
