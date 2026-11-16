// #10 E0-1: one consistent representation of notes, chords, keys and roman numerals.
import { chord, chordMidi, detectChord, diatonicChords, fromRomans, romanNumeral } from '../src/index';

describe('chord', () => {
  it('spells chords from their symbol', () => {
    expect(chord('Am7').notes).toEqual(['A', 'C', 'E', 'G']);
    expect(chord('C').notes).toEqual(['C', 'E', 'G']);
    expect(chord('G7/B').notes).toEqual(['B', 'D', 'F', 'G']); // bass first
    expect(chord('Bbmaj7').notes).toEqual(['Bb', 'D', 'F', 'A']);
  });

  it('normalises symbols to the common pop spelling', () => {
    expect(chord('CM').symbol).toBe('C');
    expect(chord('CM7').symbol).toBe('Cmaj7');
    expect(chord('Cmin7').symbol).toBe('Cm7');
    expect(chord('Bdim').symbol).toBe('Bdim');
    expect(chord('G7/B').symbol).toBe('G7/B');
  });

  it('knows root, bass and quality', () => {
    expect(chord('G7/B')).toMatchObject({ root: 'G', bass: 'B', quality: 'dominant7' });
    expect(chord('Am')).toMatchObject({ root: 'A', bass: 'A', quality: 'minor' });
    expect(chord('Csus4')).toMatchObject({ quality: 'sus4' });
  });

  it('rejects symbols it cannot read', () => {
    expect(() => chord('H#q')).toThrow(/chord/i);
  });
});

describe('chordMidi (what the piano plays and highlights)', () => {
  it('voices the chord upward from the root in the given octave', () => {
    expect(chordMidi('C', { octave: 4 })).toEqual([60, 64, 67]);
    expect(chordMidi('Am7', { octave: 3 })).toEqual([57, 60, 64, 67]);
  });

  it('plays inversions by moving the lowest notes up an octave', () => {
    expect(chordMidi('C', { octave: 4, inversion: 1 })).toEqual([64, 67, 72]);
    expect(chordMidi('C', { octave: 4, inversion: 2 })).toEqual([67, 72, 76]);
  });

  it('puts a slash-chord bass note below the chord', () => {
    expect(chordMidi('C/E', { octave: 4 })).toEqual([52, 60, 64, 67]);
  });
});

describe('detectChord', () => {
  it('prefers the reading whose root is the bass note', () => {
    expect(detectChord(['A', 'C', 'E', 'G'])[0]).toBe('Am7');
    expect(detectChord(['C', 'E', 'G', 'A'])[0]).toBe('C6');
  });

  it('names an inverted triad as a slash chord, not an exotic chord', () => {
    expect(detectChord(['E', 'G', 'C'])[0]).toBe('C/E'); // tonal alone says "Em#5"
    expect(detectChord(['G', 'C', 'E'])[0]).toBe('C/G');
  });

  it('accepts MIDI numbers and uses the lowest one as the bass', () => {
    expect(detectChord([64, 67, 72])[0]).toBe('C/E');
    expect(detectChord([57, 60, 64])[0]).toBe('Am');
  });

  it('uses the key to break ties between diatonic and chromatic readings', () => {
    // A C E G: Am7 in C, but the same notes are C6/A
    expect(detectChord(['A', 'C', 'E', 'G'], { key: 'C' })[0]).toBe('Am7');
  });

  it('returns nothing for a single note', () => {
    expect(detectChord(['C'])).toEqual([]);
  });
});

describe('romanNumeral', () => {
  it('uses lower case for minor and the common pop spelling', () => {
    expect(romanNumeral('C', 'C')).toBe('I');
    expect(romanNumeral('Dm', 'C')).toBe('ii');
    expect(romanNumeral('Am', 'C')).toBe('vi'); // tonal says "VIm"
    expect(romanNumeral('Am7', 'C')).toBe('vi7');
    expect(romanNumeral('G7', 'C')).toBe('V7');
    expect(romanNumeral('Cmaj7', 'C')).toBe('Imaj7');
    expect(romanNumeral('Bdim', 'C')).toBe('vii°');
    expect(romanNumeral('Bm7b5', 'C')).toBe('viiø7');
  });

  it('marks borrowed and chromatic chords with accidentals', () => {
    expect(romanNumeral('Bb', 'C')).toBe('♭VII');
    expect(romanNumeral('Ab', 'C')).toBe('♭VI');
    expect(romanNumeral('D', 'C')).toBe('II');
    expect(romanNumeral('F#dim', 'C')).toBe('♯iv°');
  });

  it('works in any key, including flat keys', () => {
    expect(romanNumeral('Cm', 'Eb')).toBe('vi');
    expect(romanNumeral('Bb', 'Eb')).toBe('V');
  });

  it('keeps the bass of slash chords as an inversion figure', () => {
    expect(romanNumeral('C/E', 'C')).toBe('I/3');
    expect(romanNumeral('G/B', 'C')).toBe('V/3');
    expect(romanNumeral('C/G', 'C')).toBe('I/5');
  });
});

describe('keys', () => {
  it('lists the diatonic triads and sevenths of a major key', () => {
    expect(diatonicChords('C')).toEqual(['C', 'Dm', 'Em', 'F', 'G', 'Am', 'Bdim']);
    expect(diatonicChords('C', { sevenths: true })).toEqual(['Cmaj7', 'Dm7', 'Em7', 'Fmaj7', 'G7', 'Am7', 'Bm7b5']);
  });

  it('turns roman numerals into chords in a key (round trip)', () => {
    expect(fromRomans(['I', 'V', 'vi', 'IV'], 'Eb')).toEqual(['Eb', 'Bb', 'Cm', 'Ab']);
    expect(fromRomans(['ii7', 'V7', 'Imaj7'], 'G')).toEqual(['Am7', 'D7', 'Gmaj7']);
    for (const c of diatonicChords('D', { sevenths: true })) {
      expect(fromRomans([romanNumeral(c, 'D')], 'D')).toEqual([c]);
    }
  });
});
